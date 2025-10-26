use crate::domain::areas::index::Index;
use crate::domain::areas::repository::Repository;
use crate::domain::objects::blob::Blob;
use crate::domain::objects::database_entry::DatabaseEntry;
use crate::domain::objects::file_change::{
    FileChange, FileChangeType, IndexChangeType, WorkspaceChangeType,
};
use crate::domain::objects::index_entry::{EntryMetadata, IndexEntry};
use crate::domain::objects::object::Object;
use crate::domain::objects::object_id::ObjectId;
use derive_new::new;
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

// TODO: use the file change types separately for each area change (untracked, workspace, index)
pub type FileStatSet = BTreeMap<PathBuf, EntryMetadata>;
pub type ChangeSet = BTreeMap<PathBuf, FileChangeType>;
pub type FileSet = BTreeSet<PathBuf>;

#[derive(Debug, Clone)]
pub struct StatusInfo {
    pub(crate) file_stats: FileStatSet,
    pub(crate) untracked_files: FileSet,
    pub(crate) changed_files: BTreeMap<PathBuf, FileChange>,
    pub(crate) untracked_changeset: ChangeSet,
    pub(crate) workspace_changeset: ChangeSet,
    pub(crate) index_changeset: ChangeSet,
}

#[derive(new)]
pub struct Status<'r> {
    repository: &'r Repository,
}

impl<'r> Status<'r> {
    pub async fn initialize(&self, index: &mut Index) -> anyhow::Result<StatusInfo> {
        let mut file_stats = BTreeMap::<PathBuf, EntryMetadata>::new();
        let mut untracked_files = BTreeSet::<PathBuf>::new();

        self.scan_workspace(None, &mut untracked_files, &mut file_stats, index)
            .await?;
        let head_tree = self.load_head_tree().await?;
        let mut changed_files = self.check_index_entries(&file_stats, &head_tree, index);
        self.collect_deleted_head_files(&head_tree, index, &mut changed_files);

        let untracked_changeset = untracked_files
            .iter()
            .map(|file| (file.clone(), FileChangeType::Untracked))
            .collect::<BTreeMap<_, _>>();
        let workspace_changeset = changed_files
            .iter()
            .filter(|(_, change)| change.workspace_change != WorkspaceChangeType::None)
            .map(|(file, change)| {
                (
                    file.clone(),
                    FileChangeType::Workspace(change.workspace_change.clone()),
                )
            })
            .collect::<BTreeMap<_, _>>();
        let index_changeset = changed_files
            .iter()
            .filter(|(_, change)| change.index_change != IndexChangeType::None)
            .map(|(file, change)| {
                (
                    file.clone(),
                    FileChangeType::Index(change.index_change.clone()),
                )
            })
            .collect::<BTreeMap<_, _>>();

        Ok(StatusInfo {
            file_stats,
            untracked_files,
            changed_files,
            untracked_changeset,
            workspace_changeset,
            index_changeset,
        })
    }

    async fn scan_workspace(
        &self,
        prefix_path: Option<&Path>,
        untracked_files: &mut BTreeSet<PathBuf>,
        file_stats: &mut BTreeMap<PathBuf, EntryMetadata>,
        index: &Index,
    ) -> anyhow::Result<()> {
        let files = self.repository.workspace().list_dir(prefix_path)?;

        for path in files.iter() {
            if index.is_directly_tracked(path) {
                if path.is_dir() {
                    Box::pin(self.scan_workspace(Some(path), untracked_files, file_stats, index))
                        .await?;
                } else {
                    let stat = self.repository.workspace().stat_file(path)?;
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

        if let Some(head_ref) = self.repository.refs().read_head() {
            let head_oid = ObjectId::try_parse(head_ref)?;
            let commit = self
                .repository
                .database()
                .parse_object_as_commit(&head_oid)?;

            if let Some(commit) = commit {
                self.repository
                    .parse_tree(commit.tree_oid(), None, &mut head_tree, false)
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
    ) -> BTreeMap<PathBuf, FileChange> {
        let mut changed_files = BTreeMap::<PathBuf, FileChange>::new();

        self.check_index_against_workspace(file_stats, index, &mut changed_files);
        self.check_index_against_head(head_tree, index, &mut changed_files);

        changed_files
    }

    fn check_index_against_workspace(
        &self,
        file_stats: &BTreeMap<PathBuf, EntryMetadata>,
        index: &mut Index,
        changed_files: &mut BTreeMap<PathBuf, FileChange>,
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
                        .workspace_change = WorkspaceChangeType::Deleted;

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
            changed_files.entry(path).or_default().workspace_change = WorkspaceChangeType::Modified;
        }
    }

    fn check_index_against_head(
        &self,
        head_tree: &BTreeMap<PathBuf, DatabaseEntry>,
        index: &mut Index,
        changed_files: &mut BTreeMap<PathBuf, FileChange>,
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
                    .index_change = IndexChangeType::Modified;
            } else if !head_tree.contains_key(&entry.name) {
                changed_files
                    .entry(entry.name.clone())
                    .or_default()
                    .index_change = IndexChangeType::Added;
            }
        });
    }

    fn collect_deleted_head_files(
        &self,
        head_tree: &BTreeMap<PathBuf, DatabaseEntry>,
        index: &mut Index,
        changed_files: &mut BTreeMap<PathBuf, FileChange>,
    ) {
        head_tree.iter().for_each(|(path, _)| {
            if !index.is_directly_tracked(path) {
                changed_files.entry(path.clone()).or_default().index_change =
                    IndexChangeType::Deleted;
            }
        });
    }

    fn is_content_changed(&self, index_entry: &IndexEntry) -> anyhow::Result<bool> {
        let blob = self.repository.workspace().parse_blob(&index_entry.name)?;
        let oid = blob.object_id()?;

        Ok(oid != index_entry.oid)
    }

    fn is_indirectly_tracked(&self, path: &Path, index: &Index) -> anyhow::Result<bool> {
        if path.is_file() {
            return Ok(index.is_directly_tracked(path));
        }

        let paths = self.repository.workspace().list_dir(Some(path))?;
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
}
