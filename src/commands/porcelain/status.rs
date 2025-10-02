use crate::domain::areas::index::Index;
use crate::domain::areas::repository::Repository;
use crate::domain::objects::blob::Blob;
use crate::domain::objects::database_entry::DatabaseEntry;
use crate::domain::objects::index_entry::{EntryMetadata, IndexEntry};
use crate::domain::objects::object::Object;
use crate::domain::objects::object_id::ObjectId;
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

// Terminology:
// - untracked files: files that are not tracked by the index
// - workspace modified files: files that are tracked by the index but have changes in the workspace
// - workspace deleted files: files that are tracked by the index but have been deleted from the workspace
// - index added files: files that are in the index but not in the HEAD commit
impl Repository {
    pub async fn status(&mut self) -> anyhow::Result<()> {
        let index = self.index();
        let mut index = index.lock().await;

        index.rehydrate()?;

        let mut file_stats = BTreeMap::<PathBuf, EntryMetadata>::new();
        let mut untracked_files = BTreeSet::<PathBuf>::new();

        self.scan_workspace(None, &mut untracked_files, &mut file_stats, &index)
            .await?;
        let head_tree = self.load_head_tree().await?;
        let mut changed_files = self.check_index_entries(&file_stats, &head_tree, &mut index);
        self.collect_deleted_head_files(&head_tree, &mut index, &mut changed_files);

        let changed_files = Self::coalesce_file_statuses(&changed_files);

        changed_files.iter().for_each(|(file, status)| {
            writeln!(self.writer(), "{} {}", status, file.display()).unwrap();
        });

        untracked_files.iter().for_each(|file| {
            writeln!(self.writer(), "?? {}", file.display()).unwrap();
        });

        Ok(())
    }

    async fn scan_workspace(
        &self,
        prefix_path: Option<&Path>,
        untracked_files: &mut BTreeSet<PathBuf>,
        file_stats: &mut BTreeMap<PathBuf, EntryMetadata>,
        index: &Index,
    ) -> anyhow::Result<()> {
        let files = self.workspace().list_dir(prefix_path)?;

        for path in files.iter() {
            if index.is_directly_tracked(path) {
                if path.is_dir() {
                    Box::pin(self.scan_workspace(Some(path), untracked_files, file_stats, index))
                        .await?;
                } else {
                    let stat = self.workspace().stat_file(path)?;
                    file_stats.insert(path.clone(), stat);
                }
            } else if !self.is_indirectly_tracked(path, index)? {
                // add the file separator if it's a directory
                let path = if path.is_dir() {
                    let mut p = path.clone();
                    p.push("");
                    p
                } else {
                    path.clone()
                };
                untracked_files.insert(path);
            }
        }

        Ok(())
    }

    async fn load_head_tree(&self) -> anyhow::Result<BTreeMap<PathBuf, DatabaseEntry>> {
        let mut head_tree = BTreeMap::<PathBuf, DatabaseEntry>::new();

        if let Some(head_ref) = self.refs().read_head() {
            let head_oid = ObjectId::try_parse(head_ref)?;
            let commit = self.database().parse_object_as_commit(&head_oid)?;

            if let Some(commit) = commit {
                self.parse_tree(commit.tree_oid(), None, &mut head_tree, false)
                    .await?;
            }
        }

        Ok(head_tree)
    }

    fn check_index_entries(
        &self,
        file_stats: &BTreeMap<PathBuf, EntryMetadata>,
        head_tree: &BTreeMap<PathBuf, DatabaseEntry>,
        index: &mut Index,
    ) -> BTreeMap<PathBuf, BTreeSet<FileStatus>> {
        let mut changed_files = BTreeMap::<PathBuf, BTreeSet<FileStatus>>::new();

        self.check_index_against_workspace(file_stats, index, &mut changed_files);
        self.check_index_against_head(head_tree, index, &mut changed_files);

        changed_files
    }

    fn check_index_against_workspace(
        &self,
        file_stats: &BTreeMap<PathBuf, EntryMetadata>,
        index: &mut Index,
        changed_files: &mut BTreeMap<PathBuf, BTreeSet<FileStatus>>,
    ) {
        // TODO: optimize by avoiding cloning all entries
        let index_entries = index.entries().map(Clone::clone).collect::<Vec<_>>();

        let modified_files = index_entries
            .into_iter()
            .filter_map(|entry| {
                if let Some(stat) = file_stats.get(&entry.name) {
                    Some((entry, stat))
                } else {
                    // file deleted
                    changed_files
                        .entry(entry.name.clone())
                        .or_default()
                        .insert(FileStatus::WorkspaceDeleted);
                    None
                }
            })
            .filter_map(|(index_entry, workspace_stat)| {
                match index_entry.stat_match(workspace_stat) {
                    true if index_entry.times_match(workspace_stat) => None,
                    true => self.is_content_changed(&index_entry).ok().map(|changed| {
                        if changed {
                            Some(index_entry.name.clone())
                        } else {
                            index.update_entry_stat(&index_entry, workspace_stat.clone());
                            None
                        }
                    })?,
                    false => Some(index_entry.name.clone()),
                }
            })
            .collect::<Vec<_>>();

        for path in modified_files {
            changed_files
                .entry(path)
                .or_default()
                .insert(FileStatus::WorkspaceModified);
        }
    }

    fn check_index_against_head(
        &self,
        head_tree: &BTreeMap<PathBuf, DatabaseEntry>,
        index: &mut Index,
        changed_files: &mut BTreeMap<PathBuf, BTreeSet<FileStatus>>,
    ) {
        // TODO: optimize by avoiding cloning all entries
        let index_entries = index.entries().map(Clone::clone).collect::<Vec<_>>();

        index_entries.into_iter().for_each(|entry| {
            if let Some(head_entry) = head_tree.get(&entry.name)
                && (head_entry.mode != entry.metadata.mode || head_entry.oid != entry.oid)
            {
                changed_files
                    .entry(entry.name.clone())
                    .or_default()
                    .insert(FileStatus::IndexModified);
            } else if !head_tree.contains_key(&entry.name) {
                changed_files
                    .entry(entry.name.clone())
                    .or_default()
                    .insert(FileStatus::IndexAdded);
            }
        });
    }

    fn collect_deleted_head_files(
        &self,
        head_tree: &BTreeMap<PathBuf, DatabaseEntry>,
        index: &mut Index,
        changed_files: &mut BTreeMap<PathBuf, BTreeSet<FileStatus>>,
    ) {
        head_tree.iter().for_each(|(path, _)| {
            if !index.is_directly_tracked(path) {
                changed_files
                    .entry(path.clone())
                    .or_default()
                    .insert(FileStatus::IndexDeleted);
            }
        });
    }

    fn is_content_changed(&self, index_entry: &IndexEntry) -> anyhow::Result<bool> {
        let data = self.workspace().read_file(&index_entry.name)?;
        let blob = Blob::new(data, Default::default());
        let oid = blob.object_id()?;

        Ok(oid != index_entry.oid)
    }

    fn is_indirectly_tracked(&self, path: &Path, index: &Index) -> anyhow::Result<bool> {
        if path.is_file() {
            return Ok(index.is_directly_tracked(path));
        }

        let paths = self.workspace().list_dir(Some(path))?;
        let files = paths.iter().filter(|p| p.is_file());
        let dirs = paths.iter().filter(|p| p.is_dir());

        let mut paths = files.chain(dirs);

        // chain the iterators and check if any of the files or directories are indirectly tracked
        if paths.clone().count() == 0 {
            Ok(true)
        } else {
            Ok(paths.any(|p| self.is_indirectly_tracked(p, index).unwrap_or(false)))
        }
    }

    // TODO: refactor to leverage the type system more effectively to embed the workspace and index states more naturally
    // e.g., by creating a struct that represents the combined state of the workspace and index
    fn coalesce_file_statuses(
        file_statuses: &BTreeMap<PathBuf, BTreeSet<FileStatus>>,
    ) -> BTreeMap<PathBuf, String> {
        let mut coalesced_statuses = BTreeMap::<PathBuf, String>::new();

        for (file, statuses) in file_statuses {
            let index_status = if statuses.contains(&FileStatus::IndexAdded) {
                'A'
            } else if statuses.contains(&FileStatus::IndexModified) {
                'M'
            } else if statuses.contains(&FileStatus::IndexDeleted) {
                'D'
            } else {
                ' '
            };

            let workspace_status = if statuses.contains(&FileStatus::WorkspaceDeleted) {
                'D'
            } else if statuses.contains(&FileStatus::WorkspaceModified) {
                'M'
            } else {
                ' '
            };

            coalesced_statuses.insert(
                file.clone(),
                format!("{}{}", index_status, workspace_status),
            );
        }

        coalesced_statuses
    }
}

// #[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
// enum WorkspaceChangeType {
//     Modified,
//     Deleted,
// }
//
// #[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
// enum IndexChangeType {
//     Added,
// }

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
enum FileStatus {
    IndexAdded,
    IndexModified,
    IndexDeleted,
    WorkspaceModified,
    WorkspaceDeleted,
}

impl From<&FileStatus> for &str {
    fn from(status: &FileStatus) -> Self {
        match status {
            FileStatus::IndexAdded => "A",
            FileStatus::IndexModified => "M",
            FileStatus::IndexDeleted => "D",
            FileStatus::WorkspaceModified => "M",
            FileStatus::WorkspaceDeleted => "D",
        }
    }
}

impl std::fmt::Display for FileStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let status_str: &str = self.into();
        write!(f, "{}", status_str)
    }
}
