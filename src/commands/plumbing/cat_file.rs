use crate::domain::areas::repository::Repository;
use crate::domain::objects::blob::Blob;

impl Repository {
    pub fn cat_file(&mut self, object_id: &str) -> anyhow::Result<()> {
        // read object file
        let object_data = self.database().load(object_id)?;
        let object_data = String::from_utf8(object_data.to_vec())?;

        // deserialize
        let object = Blob::new(object_data);

        write!(self.writer(), "{}", object.display())?;

        Ok(())
    }
}
