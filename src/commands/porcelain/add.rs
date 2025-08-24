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

        // Iterate over each provided file path and expand it if it's a directory
        let paths = paths
            .iter()
            .map(|path| {
                let absolute_path = Path::new(path).canonicalize()?;
                self.workspace().list_files(Some(absolute_path))
            })
            .collect::<Result<Vec<_>, _>>()?
            .into_iter()
            .flatten();

        for path in paths {
            let data = self.workspace().read_file(&path)?;
            let stat = self.workspace().stat_file(&path)?;

            let blob = Blob::new(data.as_str(), stat.clone().mode.try_into()?);
            let blob_id = blob.object_id()?;

            self.database().store(blob)?;
            index.add(IndexEntry::new(path, blob_id, stat))?;
        }

        index.write_updates()?;

        Ok(())
    }
}
