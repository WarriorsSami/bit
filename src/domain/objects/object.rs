use crate::domain::objects::object_type::ObjectType;
use crate::domain::ByteArray;
use anyhow::Result;
use sha1::{Digest, Sha1};

pub trait Object {
    fn serialize(&self) -> Result<ByteArray>;

    fn object_type(&self) -> ObjectType;

    fn object_id(&self) -> Result<String> {
        let content = self.serialize()?;
        let mut hasher = Sha1::new();
        hasher.update(&content);

        Ok(format!("{:x}", hasher.finalize()))
    }

    fn object_path(&self) -> Result<String> {
        let object_id = self.object_id()?;

        Ok(format!(
            "{}/{}",
            &object_id[..2],
            &object_id[2..]
        ))
    }
}
