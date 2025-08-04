use crate::domain::areas::repository::Repository;
use crate::domain::objects::blob::Blob;
use crate::domain::objects::index_entry::IndexEntry;
use crate::domain::objects::object::Object;
use std::path::Path;

impl Repository {
    pub async fn add(&mut self, paths: &[String]) -> anyhow::Result<()> {
        let path = Path::new(&paths[0]);

        let data = self.workspace().read_file(path)?;
        let stat = self.workspace().stat_file(path)?;

        let blob = Blob::new(data.as_str(), stat.clone().mode.try_into()?);
        let blob_id = blob.object_id()?;

        self.database().store(blob)?;

        let index = self.index();
        let mut index = index.lock().await;
        index.add(IndexEntry::new(path.into(), blob_id, stat))?;

        index.write_updates()?;

        Ok(())
    }
}
