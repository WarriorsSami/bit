use crate::domain::blob_object::BlobObject;
use crate::domain::git_object::GitObject;
use crate::domain::git_repository::Repository;
use anyhow::Context;
use std::fs;

impl Repository {
    pub fn cat_file(&mut self, object_id: &str) -> anyhow::Result<()> {
        // read object file
        let object_path = self.object_path(object_id);
        let object_data = fs::read(&object_path).context("Unable to read object file")?;

        // decompress
        let object_data = BlobObject::decompress(object_data.into())?;

        // deserialize
        let object = BlobObject::deserialize(object_data)?;

        write!(self.writer, "{}", object.pretty_print())?;

        Ok(())
    }
}
