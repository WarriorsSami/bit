use crate::domain::blob_object::BlobObject;
use crate::domain::git_object::GitObject;
use crate::domain::git_repository::Repository;
use anyhow::Context;
use std::fs;

impl Repository {
    pub fn hash_object(&mut self, object_path: &str, write: bool) -> anyhow::Result<()> {
        // read object file
        let object_data = fs::read(object_path).context("Unable to read object file")?;
        let object = BlobObject::new(object_data.into());

        // hash
        let object_id = object.object_id()?;

        write!(self.writer, "{}", object_id)?;

        // write (if write is true) as compressed object file
        if !write {
            return Ok(());
        }

        let object_path = self.object_path(&object_id); 

        let object_data = BlobObject::compress(&object)?;

        if !fs::metadata(&object_path)
            .map(|m| m.permissions().readonly() || !m.is_file())
            .context("The object file is read-only, is not a file or does not exist")?
        {
            fs::write(&object_path, &object_data).context("Unable to write object file")?;
        }

        Ok(())
    }
}
