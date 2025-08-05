use crate::domain::objects::object_id::ObjectId;
use crate::domain::objects::object_type::ObjectType;
use anyhow::Result;
use bytes::Bytes;
use sha1::{Digest, Sha1};
use std::path::PathBuf;

pub trait Packable {
    fn serialize(&self) -> Result<Bytes>;
}

pub trait Object: Packable {
    fn object_type(&self) -> ObjectType;

    fn display(&self) -> String;

    fn object_id(&self) -> Result<ObjectId> {
        let content = self.serialize()?;
        let mut hasher = Sha1::new();
        hasher.update(&content);

        let oid = hasher.finalize();
        ObjectId::try_parse(format!("{oid:x}"))
    }

    fn object_path(&self) -> Result<PathBuf> {
        let object_id = self.object_id()?;
        let object_id = object_id.as_ref();

        Ok(PathBuf::from(format!(
            "{}/{}",
            &object_id[..2],
            &object_id[2..]
        )))
    }
}
