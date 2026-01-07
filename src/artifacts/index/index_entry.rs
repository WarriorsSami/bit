//! Index entry representation
//!
//! Each entry in the index represents a tracked file with:
//! - File path
//! - Content hash (object ID)
//! - File metadata (mode, size, timestamps)
//!
//! ## Entry Format
//!
//! Entries are stored in a binary format with 8-byte alignment for efficient reading.
//! Metadata includes both file status (mode, size) and timestamps (mtime, ctime)
//! which enable fast change detection without reading file content.

use crate::artifacts::index::entry_mode::{EntryMode, FileMode};
use crate::artifacts::objects::object::{Packable, Unpackable};
use crate::artifacts::objects::object_id::ObjectId;
use byteorder::{ByteOrder, WriteBytesExt};
use bytes::Bytes;
use derive_new::new;
use is_executable::IsExecutable;
use std::cmp::min;
use std::fs::Metadata;
use std::io::{BufRead, Write};
use std::os::unix::prelude::MetadataExt;
use std::path::{Path, PathBuf};

/// Maximum path length supported in index entries
const MAX_PATH_SIZE: usize = 4095;

/// Block size for entry alignment (8 bytes)
pub const ENTRY_BLOCK: usize = 8;

/// Minimum size of an index entry in bytes
pub const ENTRY_MIN_SIZE: usize = 64; // Minimum size of an index entry in bytes

/// Index entry representing a tracked file
///
/// Contains the file path, content hash, and metadata needed for
/// efficient change detection.
// TODO: Restrict access to certain fields
#[derive(Debug, Clone, Default, new)]
pub struct IndexEntry {
    /// File path relative to repository root
    pub name: PathBuf,
    /// SHA-1 hash of file content
    pub oid: ObjectId,
    /// File metadata (mode, size, timestamps)
    pub metadata: EntryMetadata,
}

impl IndexEntry {
    pub fn basename(&self) -> anyhow::Result<&str> {
        self.name
            .file_name()
            .and_then(|name| name.to_str())
            .ok_or_else(|| anyhow::anyhow!("Invalid file name"))
    }

    // TODO: Stop after reaching the repository's root
    pub fn parent_dirs(&self) -> anyhow::Result<Vec<&Path>> {
        let mut dirs = Vec::new();
        let mut parent = self.name.parent();

        while let Some(new_parent) = parent {
            dirs.push(new_parent);
            parent = new_parent.parent();
        }
        dirs.reverse();
        let dirs = dirs[1..].to_vec();

        Ok(dirs)
    }

    pub fn stat_match(&self, other: &EntryMetadata) -> bool {
        (self.metadata.size == 0 || self.metadata.size == other.size)
            && self.metadata.mode == other.mode
    }

    pub fn times_match(&self, other: &EntryMetadata) -> bool {
        self.metadata.ctime == other.ctime
            && self.metadata.ctime_nsec == other.ctime_nsec
            && self.metadata.mtime == other.mtime
            && self.metadata.mtime_nsec == other.mtime_nsec
    }
}

impl PartialEq for IndexEntry {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name
    }
}

impl Eq for IndexEntry {}

impl PartialOrd for IndexEntry {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for IndexEntry {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.name.cmp(&other.name)
    }
}

/// File metadata stored in index entries
///
/// Contains both file status information (mode, size, inode) and timestamps.
/// This metadata enables Git to quickly detect file changes without reading
/// content by comparing stat information.
///
/// ## Timestamps
///
/// - `ctime`: File status change time (inode modification)
/// - `mtime`: File content modification time
///
/// Both include nanosecond precision for accurate change detection.
#[derive(Debug, Clone, Default)]
pub struct EntryMetadata {
    /// Change time (seconds since Unix epoch)
    pub ctime: i64,
    /// Change time nanoseconds
    pub ctime_nsec: i64,
    /// Modification time (seconds since Unix epoch)
    pub mtime: i64,
    /// Modification time nanoseconds
    pub mtime_nsec: i64,
    /// Device ID
    pub dev: u64,
    /// Inode number
    pub ino: u64,
    /// File mode (permissions and type)
    pub mode: EntryMode,
    /// User ID of owner
    pub uid: u32,
    /// Group ID of owner
    pub gid: u32,
    /// File size in bytes
    pub size: u64,
    /// Entry flags (reserved for future use)
    pub flags: u32,
}

impl Packable for IndexEntry {
    fn serialize(&self) -> anyhow::Result<Bytes> {
        let entry_name = String::from(
            self.name
                .to_str()
                .ok_or_else(|| anyhow::anyhow!("Invalid entry name"))?,
        );
        let entry_mode = self.metadata.mode.as_u32();

        let mut entry_bytes = Vec::new();
        entry_bytes.write_u32::<byteorder::NetworkEndian>(self.metadata.ctime as u32)?;
        entry_bytes.write_u32::<byteorder::NetworkEndian>(self.metadata.ctime_nsec as u32)?;
        entry_bytes.write_u32::<byteorder::NetworkEndian>(self.metadata.mtime as u32)?;
        entry_bytes.write_u32::<byteorder::NetworkEndian>(self.metadata.mtime_nsec as u32)?;
        entry_bytes.write_u32::<byteorder::NetworkEndian>(self.metadata.dev as u32)?;
        entry_bytes.write_u32::<byteorder::NetworkEndian>(self.metadata.ino as u32)?;
        entry_bytes.write_u32::<byteorder::NetworkEndian>(entry_mode)?;
        entry_bytes.write_u32::<byteorder::NetworkEndian>(self.metadata.uid)?;
        entry_bytes.write_u32::<byteorder::NetworkEndian>(self.metadata.gid)?;
        entry_bytes.write_u32::<byteorder::NetworkEndian>(self.metadata.size as u32)?;
        self.oid.write_h40_to(&mut entry_bytes)?;
        entry_bytes.write_u16::<byteorder::NetworkEndian>(self.metadata.flags as u16)?;
        entry_bytes.write_all(entry_name.as_bytes())?;

        // Ensure the entry bytes are padded to ENTRY_BLOCK size with null bytes
        entry_bytes.push(0); // There must be at least one null byte at the end
        while entry_bytes.len() % ENTRY_BLOCK != 0 {
            entry_bytes.push(0);
        }

        Ok(Bytes::from(entry_bytes))
    }
}

impl Unpackable for IndexEntry {
    fn deserialize(reader: impl BufRead) -> anyhow::Result<Self> {
        let bytes = reader
            .bytes()
            .collect::<Result<Vec<u8>, std::io::Error>>()?;

        if bytes.len() < ENTRY_MIN_SIZE {
            return Err(anyhow::anyhow!("Invalid index entry size"));
        }

        let ctime = byteorder::NetworkEndian::read_u32(&bytes[0..4]) as i64;
        let ctime_nsec = byteorder::NetworkEndian::read_u32(&bytes[4..8]) as i64;
        let mtime = byteorder::NetworkEndian::read_u32(&bytes[8..12]) as i64;
        let mtime_nsec = byteorder::NetworkEndian::read_u32(&bytes[12..16]) as i64;
        let dev = byteorder::NetworkEndian::read_u32(&bytes[16..20]) as u64;
        let ino = byteorder::NetworkEndian::read_u32(&bytes[20..24]) as u64;
        let mode: EntryMode = byteorder::NetworkEndian::read_u32(&bytes[24..28]).into();
        let uid = byteorder::NetworkEndian::read_u32(&bytes[28..32]);
        let gid = byteorder::NetworkEndian::read_u32(&bytes[32..36]);
        let size = byteorder::NetworkEndian::read_u32(&bytes[36..40]) as u64;
        let mut oid_bytes = std::io::Cursor::new(&bytes[40..60]);
        let oid = ObjectId::read_h40_from(&mut oid_bytes)?;
        let flags = byteorder::NetworkEndian::read_u16(&bytes[60..62]) as u32;

        // Extract the entry name, which is null-terminated
        let name_end = bytes[62..]
            .iter()
            .position(|&b| b == 0)
            .ok_or_else(|| anyhow::anyhow!("Missing null terminator in entry name"))?;
        let name_bytes = &bytes[62..62 + name_end];
        let name = PathBuf::from(
            std::str::from_utf8(name_bytes)
                .map_err(|_| anyhow::anyhow!("Invalid UTF-8 in entry name"))?,
        );

        Ok(IndexEntry {
            name,
            oid,
            metadata: EntryMetadata {
                ctime,
                ctime_nsec,
                mtime,
                mtime_nsec,
                dev,
                ino,
                mode,
                uid,
                gid,
                size,
                flags,
            },
        })
    }
}

impl TryFrom<(&Path, Metadata)> for EntryMetadata {
    type Error = anyhow::Error;

    fn try_from((file_path, metadata): (&Path, Metadata)) -> Result<Self, Self::Error> {
        let mode = if metadata.is_dir() {
            EntryMode::Directory
        } else {
            match file_path.is_executable() {
                true => EntryMode::File(FileMode::Executable),
                false => EntryMode::File(FileMode::Regular),
            }
        };
        let file_path = file_path
            .to_str()
            .ok_or_else(|| anyhow::anyhow!("Invalid file path"))?;

        Ok(Self {
            ctime: metadata.ctime(),
            ctime_nsec: metadata.ctime_nsec(),
            mtime: metadata.mtime(),
            mtime_nsec: metadata.mtime_nsec(),
            dev: metadata.dev(),
            ino: metadata.ino(),
            mode,
            uid: metadata.uid(),
            gid: metadata.gid(),
            size: metadata.size(),
            flags: min(file_path.len(), MAX_PATH_SIZE) as u32,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::{fixture, rstest};
    use sha1::Digest;

    #[fixture]
    fn oid() -> ObjectId {
        let mut hasher = sha1::Sha1::new();
        hasher.update("test data");
        ObjectId::try_parse(format!("{:x}", hasher.finalize())).unwrap()
    }

    #[fixture]
    fn entry_metadata() -> EntryMetadata {
        EntryMetadata {
            mode: EntryMode::Directory,
            ..Default::default()
        }
    }

    #[rstest]
    fn test_entry_parent_dirs(oid: ObjectId, entry_metadata: EntryMetadata) {
        let entry = IndexEntry::new(PathBuf::from("a/b/c"), oid, entry_metadata);

        let dirs = entry.parent_dirs().unwrap();
        pretty_assertions::assert_eq!(dirs, vec![Path::new("a"), Path::new("a/b")]);
    }

    #[rstest]
    fn test_entry_parent_dirs_root(oid: ObjectId, entry_metadata: EntryMetadata) {
        let entry = IndexEntry::new(PathBuf::from("a"), oid, entry_metadata);

        let dirs = entry.parent_dirs().unwrap();
        pretty_assertions::assert_eq!(dirs, Vec::<&Path>::new());
    }

    #[rstest]
    fn test_entry_basename(oid: ObjectId, entry_metadata: EntryMetadata) {
        let entry = IndexEntry::new(PathBuf::from("a/b/c"), oid, entry_metadata);

        let basename = entry.basename().unwrap();
        pretty_assertions::assert_eq!(basename, "c");
    }
}
