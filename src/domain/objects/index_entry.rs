use crate::domain::objects::entry_mode::{EntryMode, FileMode};
use crate::domain::objects::object::Packable;
use crate::domain::objects::object_id::ObjectId;
use byteorder::WriteBytesExt;
use bytes::Bytes;
use derive_new::new;
use is_executable::IsExecutable;
use std::cmp::min;
use std::fs::Metadata;
use std::io::Write;
use std::os::unix::prelude::MetadataExt;
use std::path::{Path, PathBuf};

const MAX_PATH_SIZE: usize = 4095;
const ENTRY_BLOCK: usize = 8;

#[derive(Debug, Clone, Default, new)]
pub struct IndexEntry {
    pub name: PathBuf,
    pub oid: ObjectId,
    pub metadata: EntryMetadata,
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
