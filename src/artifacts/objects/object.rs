use crate::artifacts::objects::blob::Blob;
use crate::artifacts::objects::commit::Commit;
use crate::artifacts::objects::object_id::ObjectId;
use crate::artifacts::objects::object_type::ObjectType;
use crate::artifacts::objects::tree::Tree;
use anyhow::Result;
use bytes::Bytes;
use sha1::{Digest, Sha1};
use std::io::BufRead;
use std::path::PathBuf;

// TODO: Consider mutably borrowing BufReader and BufWriter for efficiency
pub trait Packable {
    fn serialize(&self) -> Result<Bytes>;
}

pub trait Unpackable {
    fn deserialize(reader: impl BufRead) -> Result<Self>
    where
        Self: Sized;
}

pub trait Object: Packable {
    fn object_type(&self) -> ObjectType;

    fn display(&self) -> String;

    // TODO: Cache the object serialization and ID to avoid recomputing them
    fn object_id(&self) -> Result<ObjectId> {
        let content = self.serialize()?;
        let mut hasher = Sha1::new();
        hasher.update(&content);

        let oid = hasher.finalize();
        ObjectId::try_parse(format!("{oid:x}"))
    }

    fn object_path(&self) -> Result<PathBuf> {
        Ok(self.object_id()?.to_path())
    }
}

pub enum ObjectBox<'o> {
    Blob(Box<Blob>),
    Tree(Box<Tree<'o>>),
    Commit(Box<Commit>),
}
