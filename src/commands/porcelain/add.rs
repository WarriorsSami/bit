use crate::domain::areas::repository::Repository;
use crate::domain::objects::blob::Blob;
use crate::domain::objects::index_entry::IndexEntry;
use crate::domain::objects::object::Object;
use std::path::Path;

impl Repository {
    pub async fn add(&mut self, paths: &[String]) -> anyhow::Result<()> {
        let index = self.index();
        let mut index = index.lock().await;

        // Load the index file from the disk
        index.rehydrate()?;

        for file_path in paths {
            // Convert the file path to an absolute path using `canonicalize`
            let absolute_path = Path::new(&file_path).canonicalize()?;

            for path in self.workspace().list_files(Some(absolute_path)) {
                let data = self.workspace().read_file(&path);

                if data.is_err() {
                    // If the file does not exist or cannot be read, skip it
                    continue;
                }
                let data = data?;

                let stat = self.workspace().stat_file(&path)?;

                let blob = Blob::new(data.as_str(), stat.clone().mode.try_into()?);
                let blob_id = blob.object_id()?;

                self.database().store(blob)?;
                index.add(IndexEntry::new(path, blob_id, stat))?;
            }
        }

        index.write_updates()?;

        Ok(())
    }
}
