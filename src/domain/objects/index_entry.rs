use crate::domain::objects::entry_mode::{EntryMode, FileMode};
use is_executable::IsExecutable;
use std::cmp::min;
use std::fs::Metadata;
use std::os::unix::prelude::MetadataExt;
use std::path::PathBuf;
use derive_new::new;

const MAX_PATH_SIZE: usize = 4095;

// TODO: Define a dedicated IndexEntry type for the index and keep the previous entry type for the commit tree.
#[derive(Debug, Clone, Eq, Ord, Default, PartialEq, PartialOrd, new)]
pub struct IndexEntry {
    pub name: PathBuf,
    pub oid: String,
    pub metadata: EntryMetadata,
}

#[derive(Debug, Clone, Eq, Ord, Default, PartialEq, PartialOrd)]
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

impl TryFrom<(&PathBuf, Metadata)> for EntryMetadata {
    type Error = anyhow::Error;

    fn try_from((file_path, metadata): (&PathBuf, Metadata)) -> Result<Self, Self::Error> {
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
