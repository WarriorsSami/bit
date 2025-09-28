use crate::domain::areas::index::Index;
use crate::domain::areas::repository::Repository;
use crate::domain::objects::blob::Blob;
use crate::domain::objects::index_entry::{EntryMetadata, IndexEntry};
use crate::domain::objects::object::Object;
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

// Terminology:
// - untracked files: files that are not tracked by the index
// - changed/modified files: files that are tracked by the index but have changes in the workspace
// - deleted files: files that are tracked by the index but have been deleted from the workspace
impl Repository {
    pub async fn status(&mut self) -> anyhow::Result<()> {
        let index = self.index();
        let mut index = index.lock().await;
        index.rehydrate()?;

        let mut file_stats = BTreeMap::<PathBuf, EntryMetadata>::new();
        let mut untracked_files = BTreeSet::<PathBuf>::new();

        self.scan_workspace(None, &mut untracked_files, &mut file_stats, &index)
            .await?;
        let changed_files = self.detect_workspace_changes(&file_stats, &mut index);

        index.write_updates()?;

        changed_files.iter().for_each(|(file, status)| {
            writeln!(self.writer(), " {} {}", status, file.display()).unwrap();
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

    fn detect_workspace_changes(
        &self,
        file_stats: &BTreeMap<PathBuf, EntryMetadata>,
        index: &mut Index,
    ) -> BTreeMap<PathBuf, FileStatus> {
        let mut changed_files = BTreeMap::<PathBuf, BTreeSet<FileStatus>>::new();

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
                        .insert(FileStatus::Deleted);
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
                .insert(FileStatus::Modified);
        }

        Self::coalesce_file_statuses(&changed_files)
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

    fn coalesce_file_statuses(
        file_statuses: &BTreeMap<PathBuf, BTreeSet<FileStatus>>,
    ) -> BTreeMap<PathBuf, FileStatus> {
        let mut coalesced_statuses = BTreeMap::<PathBuf, FileStatus>::new();

        for (file, statuses) in file_statuses {
            if statuses.contains(&FileStatus::Deleted) {
                coalesced_statuses.insert(file.clone(), FileStatus::Deleted);
            }

            if statuses.contains(&FileStatus::Modified) {
                coalesced_statuses.insert(file.clone(), FileStatus::Modified);
            }
        }

        coalesced_statuses
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
enum FileStatus {
    Modified,
    Deleted,
}

impl From<&FileStatus> for &str {
    fn from(status: &FileStatus) -> Self {
        match status {
            FileStatus::Modified => "M",
            FileStatus::Deleted => "D",
        }
    }
}

impl std::fmt::Display for FileStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let status_str: &str = self.into();
        write!(f, "{}", status_str)
    }
}
