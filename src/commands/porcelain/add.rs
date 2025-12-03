use crate::areas::repository::Repository;
use crate::artifacts::index::index_entry::IndexEntry;
use crate::artifacts::objects::blob::Blob;
use crate::artifacts::objects::object::Object;
use std::collections::HashSet;
use std::path::PathBuf;

impl Repository {
    pub async fn add(&mut self, paths: &[String]) -> anyhow::Result<()> {
        let index = self.index();
        let mut index = index.lock().await;

        // Load the index file from the disk
        index.rehydrate()?;

        // Iterate over each provided file path and expand it if it's a directory
        let paths = paths
            .iter()
            .map(|path| (path, self.workspace().list_files(Some(PathBuf::from(path)))))
            .collect::<Vec<_>>();

        // Collect all the invalid paths to remove them from the index in case they were tracked before
        let invalid_paths = paths
            .iter()
            .filter_map(
                |(path, files)| {
                    if files.is_err() { Some(path) } else { None }
                },
            )
            .collect::<Vec<_>>();
        for path in invalid_paths.iter() {
            index.remove(PathBuf::from(path))?;
        }

        if !invalid_paths.is_empty() {
            index.write_updates()?;
            anyhow::bail!("The following paths are not valid: {:?}", invalid_paths);
        }

        // Collect all the valid paths to add them to the index
        let valid_paths = paths
            .iter()
            .filter_map(
                |(_path, files)| {
                    if files.is_ok() { Some(files) } else { None }
                },
            )
            .flatten()
            .flatten()
            .collect::<Vec<_>>();

        // Collect workspace files into a set for deletion check later
        let workspace_files: HashSet<PathBuf> = valid_paths.iter().map(|p| (*p).clone()).collect();

        for path in &valid_paths {
            let data = self.workspace().read_file(path)?;
            let stat = self.workspace().stat_file(path)?;

            let blob = Blob::new(data, stat.clone().mode.try_into()?);
            let blob_id = blob.object_id()?;

            self.database().store(blob)?;
            index.add(IndexEntry::new(path.to_path_buf(), blob_id, stat))?;
        }

        // Handle deletions: Check if tracked files in the index no longer exist in the workspace
        // For each provided path, get all tracked files under that path from the index
        // and remove ones that don't exist in the workspace
        for path_str in paths
            .iter()
            .filter_map(|(p, files)| if files.is_ok() { Some(*p) } else { None })
        {
            let path = PathBuf::from(path_str);
            let tracked_files = index.entries_under_path(&path);

            for tracked_file in tracked_files {
                if !workspace_files.contains(&tracked_file) {
                    // File was tracked but no longer exists in workspace - remove it
                    index.remove(tracked_file)?;
                }
            }
        }

        index.write_updates()?;

        Ok(())
    }
}
