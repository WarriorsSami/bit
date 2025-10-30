use crate::domain::areas::repository::Repository;
use crate::domain::objects::blob::Blob;
use crate::domain::objects::index_entry::IndexEntry;
use crate::domain::objects::object::Object;
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

        for path in valid_paths {
            let data = self.workspace().read_file(path)?;
            let stat = self.workspace().stat_file(path)?;

            let blob = Blob::new(data, stat.clone().mode.try_into()?);
            let blob_id = blob.object_id()?;

            self.database().store(blob)?;
            index.add(IndexEntry::new(path.to_path_buf(), blob_id, stat))?;
        }

        index.write_updates()?;

        Ok(())
    }
}
