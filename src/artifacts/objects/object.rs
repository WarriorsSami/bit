//! Core object traits and types
//!
//! This module defines the fundamental traits that all Git objects implement:
//! - `Packable`: Serialization to Git's binary format
//! - `Unpackable`: Deserialization from Git's binary format
//! - `Object`: Common object operations (ID computation, display)
//!
//! ## Object Format
//!
//! All objects are stored as:
//! ```text
//! <type> <size>\0<content>
//! ```
//! Then compressed with zlib and stored in `.git/objects/`.

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

/// Trait for serializing objects to Git's binary format
// TODO: Consider mutably borrowing BufReader and BufWriter for efficiency
pub trait Packable {
    /// Serialize the object to bytes (including header)
    fn serialize(&self) -> Result<Bytes>;
}

/// Trait for deserializing objects from Git's binary format
pub trait Unpackable {
    /// Deserialize the object from a reader (header already consumed)
    fn deserialize(reader: impl BufRead) -> Result<Self>
    where
        Self: Sized;
}

/// Core Git object trait
///
/// Implemented by all Git object types (Blob, Tree, Commit).
/// Provides common operations like ID computation and display.
pub trait Object: Packable {
    /// Get the object's type
    fn object_type(&self) -> ObjectType;

    /// Get a human-readable representation
    fn display(&self) -> String;

    /// Compute the object ID (SHA-1 hash)
    ///
    /// The ID is computed by hashing the serialized content.
    // TODO: Cache the object serialization and ID to avoid recomputing them
    fn object_id(&self) -> Result<ObjectId> {
        let content = self.serialize()?;
        let mut hasher = Sha1::new();
        hasher.update(&content);

        let oid = hasher.finalize();
        ObjectId::try_parse(format!("{oid:x}"))
    }

    /// Get the file system path where this object would be stored
    fn object_path(&self) -> Result<PathBuf> {
        Ok(self.object_id()?.to_path())
    }
}

/// Type-erased object container
///
/// Used when the specific object type isn't known at compile time.
/// Allows returning different object types from a single function.
pub enum ObjectBox<'o> {
    Blob(Box<Blob>),
    Tree(Box<Tree<'o>>),
    Commit(Box<Commit>),
}
