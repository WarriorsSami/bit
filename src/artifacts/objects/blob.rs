//! Git blob object
//!
//! Blobs store file content in Git. They contain only the raw file data,
//! without any metadata like filename or permissions (those are stored in trees).
//!
//! ## Format
//!
//! On disk: `blob <size>\0<content>`
//! In memory: Just the content string and file mode

use crate::artifacts::index::entry_mode::FileMode;
use crate::artifacts::objects::object::Unpackable;
use crate::artifacts::objects::object::{Object, Packable};
use crate::artifacts::objects::object_type::ObjectType;
use bytes::Bytes;
use derive_new::new;
use std::io::{BufRead, Write};

/// Git blob object representing file content
///
/// Blobs are the fundamental unit of file storage in Git.
/// Each unique file content is stored as a blob, identified by its SHA-1 hash.
#[derive(Debug, Clone, new)]
pub struct Blob {
    /// File content as a string
    content: String,
    /// File mode (permissions)
    stat: FileMode,
}

impl Blob {
    /// Get the file mode (permissions)
    pub fn mode(&self) -> &FileMode {
        &self.stat
    }

    /// Get the file content as a string
    pub fn content(&self) -> &str {
        &self.content
    }
}

impl Packable for Blob {
    fn serialize(&self) -> anyhow::Result<Bytes> {
        let mut content_bytes = Vec::new();
        content_bytes.write_all(self.content.as_bytes())?;

        let mut blob_bytes = Vec::new();
        let header = format!("{} {}\0", self.object_type().as_str(), content_bytes.len());
        blob_bytes.write_all(header.as_bytes())?;
        blob_bytes.write_all(&content_bytes)?;

        Ok(Bytes::from(blob_bytes))
    }
}

impl Unpackable for Blob {
    fn deserialize(reader: impl BufRead) -> anyhow::Result<Self> {
        // the header has already been read
        let content = reader
            .bytes()
            .collect::<Result<Vec<u8>, std::io::Error>>()?;

        let content = String::from_utf8(content)?;
        Ok(Self::new(content, Default::default()))
    }
}

impl Object for Blob {
    fn object_type(&self) -> ObjectType {
        ObjectType::Blob
    }

    fn display(&self) -> String {
        self.content.to_string()
    }
}
