use crate::domain::areas::repository::Repository;
use crate::domain::objects::blob::Blob;
use crate::domain::objects::object::Object;

impl Repository {
    pub fn hash_object(&mut self, object_path: &str, write: bool) -> anyhow::Result<()> {
        // read the object file
        let object_data = self.workspace().read_file(object_path.as_ref())?;
        let object = Blob::new(object_data, Default::default());

        // hash
        let object_id = object.object_id()?;

        write!(self.writer(), "{}", object_id.as_ref())?;

        // write (if write is true) as a compressed object file
        if !write {
            return Ok(());
        }

        self.database().store(object)?;

        Ok(())
    }
}
