use crate::domain::areas::index::Index;
use crate::domain::areas::repository::Repository;
use crate::domain::objects::index_entry::EntryMetadata;
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

// Terminology:
// - untracked files: files that are not tracked by the index
// - changed/modified files: files that are tracked by the index but have changes in the workspace
impl Repository {
    pub async fn status(&mut self) -> anyhow::Result<()> {
        let index = self.index();
        let mut index = index.lock().await;
        index.rehydrate()?;

        let mut file_stats = BTreeMap::<PathBuf, EntryMetadata>::new();
        let mut untracked_files = BTreeSet::<PathBuf>::new();

        self.scan_workspace(None, &mut untracked_files, &mut file_stats, &index)
            .await?;
        let changed_files = Self::detect_workspace_changes(&file_stats, &index);

        changed_files.iter().for_each(|file| {
            writeln!(self.writer(), " M {}", file.display()).unwrap();
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
        file_stats: &BTreeMap<PathBuf, EntryMetadata>,
        index: &Index,
    ) -> BTreeSet<PathBuf> {
        index
            .entries()
            .filter_map(|entry| file_stats.get(&entry.name).map(|stat| (entry, stat)))
            .filter_map(|(index_entry, workspace_stat)| {
                match index_entry.stat_match(workspace_stat) {
                    true => None,
                    false => Some(index_entry.name.clone()),
                }
            })
            .collect()
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
}
