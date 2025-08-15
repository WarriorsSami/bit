use crate::domain::objects::entry_mode::{EntryMode, FileMode};
use crate::domain::objects::object::{Packable, Unpackable};
use crate::domain::objects::object_id::ObjectId;
use byteorder::{ByteOrder, WriteBytesExt};
use bytes::Bytes;
use derive_new::new;
use is_executable::IsExecutable;
use std::cmp::min;
use std::fs::Metadata;
use std::io::Write;
use std::os::unix::prelude::MetadataExt;
use std::path::{Path, PathBuf};

const MAX_PATH_SIZE: usize = 4095;
pub const ENTRY_BLOCK: usize = 8;
pub const ENTRY_MIN_SIZE: usize = 64; // Minimum size of an index entry in bytes

#[derive(Debug, Clone, Default, new)]
pub struct IndexEntry {
    pub name: PathBuf,
    pub oid: ObjectId,
    pub metadata: EntryMetadata,
}

impl IndexEntry {
    pub fn basename(&self) -> anyhow::Result<&str> {
        self.name
            .file_name()
            .and_then(|name| name.to_str())
            .ok_or_else(|| anyhow::anyhow!("Invalid file name"))
    }

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

#[derive(Debug, Clone, Default)]
pub struct EntryMetadata {
    pub ctime: i64,
    pub ctime_nsec: i64,
    pub mtime: i64,
    pub mtime_nsec: i64,
    pub dev: u64,
    pub ino: u64,
    pub mode: EntryMode,
    pub uid: u32,
    pub gid: u32,
    pub size: u64,
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
        // pack!(
        //     self.metadata.ctime,
        //     self.metadata.ctime_nsec,
        //     self.metadata.mtime,
        //     self.metadata.mtime_nsec,
        //     self.metadata.dev,
        //     self.metadata.ino,
        //     entry_mode,
        //     self.metadata.uid,
        //     self.metadata.gid,
        //     self.metadata.size,
        //     self.oid,
        //     self.metadata.flags,
        //     entry_name
        //     => "N10H40nZ"
        // );
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
    fn deserialize(bytes: Bytes) -> anyhow::Result<Self> {
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
        let name_end = bytes
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
