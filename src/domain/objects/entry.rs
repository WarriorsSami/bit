use is_executable::IsExecutable;
use std::cmp::min;
use std::fs::Metadata;
use std::os::unix::prelude::MetadataExt;
use std::path::{Path, PathBuf};

const MAX_PATH_SIZE: usize = 4095;

// TODO: Define a dedicated IndexEntry type for the index and keep the previous entry type for the commit tree.
#[derive(Debug, Clone, Eq, Ord, Default, PartialEq, PartialOrd)]
pub struct Entry {
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

impl Entry {
    pub fn new(name: PathBuf, oid: String, metadata: EntryMetadata) -> Self {
        Self {
            name,
            oid,
            metadata,
        }
    }

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

#[derive(Debug, Clone, Eq, Ord, Default, PartialEq, PartialOrd)]
pub enum FileMode {
    #[default]
    Regular,
    Executable,
}

#[derive(Debug, Clone, Eq, Ord, Default, PartialEq, PartialOrd)]
pub enum EntryMode {
    File(FileMode),
    #[default]
    Directory,
}

impl EntryMode {
    pub fn as_str(&self) -> &str {
        match self {
            EntryMode::File(FileMode::Regular) => "100644",
            EntryMode::File(FileMode::Executable) => "100755",
            EntryMode::Directory => "40000",
        }
    }

    pub fn as_u32(&self) -> u32 {
        match self {
            EntryMode::File(FileMode::Regular) => 0o100644,
            EntryMode::File(FileMode::Executable) => 0o100755,
            EntryMode::Directory => 0o40000,
        }
    }
}

impl From<FileMode> for EntryMode {
    fn from(mode: FileMode) -> Self {
        EntryMode::File(mode)
    }
}

impl From<&FileMode> for &EntryMode {
    fn from(mode: &FileMode) -> Self {
        match mode {
            FileMode::Regular => &EntryMode::File(FileMode::Regular),
            FileMode::Executable => &EntryMode::File(FileMode::Executable),
        }
    }
}

impl TryFrom<EntryMode> for FileMode {
    type Error = anyhow::Error;

    fn try_from(value: EntryMode) -> anyhow::Result<Self> {
        match value {
            EntryMode::File(FileMode::Regular) => Ok(FileMode::Regular),
            EntryMode::File(FileMode::Executable) => Ok(FileMode::Executable),
            _ => Err(anyhow::anyhow!("Invalid entry mode")),
        }
    }
}

impl TryFrom<&str> for EntryMode {
    type Error = anyhow::Error;

    fn try_from(value: &str) -> anyhow::Result<Self> {
        match value {
            "100644" => Ok(EntryMode::File(FileMode::Regular)),
            "100755" => Ok(EntryMode::File(FileMode::Executable)),
            "40000" => Ok(EntryMode::Directory),
            _ => Err(anyhow::anyhow!("Invalid entry mode")),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::{fixture, rstest};

    #[fixture]
    fn entry_metadata() -> EntryMetadata {
        EntryMetadata {
            ctime: 0,
            ctime_nsec: 0,
            mtime: 0,
            mtime_nsec: 0,
            dev: 0,
            ino: 0,
            mode: EntryMode::Directory,
            uid: 0,
            gid: 0,
            size: 0,
            flags: 0,
        }
    }

    #[rstest]
    fn test_entry_parent_dirs(entry_metadata: EntryMetadata) {
        let entry = Entry::new(PathBuf::from("a/b/c"), "".to_string(), entry_metadata);

        let dirs = entry.parent_dirs().unwrap();
        assert_eq!(dirs, vec![Path::new("a"), Path::new("a/b")]);
    }

    #[rstest]
    fn test_entry_parent_dirs_root(entry_metadata: EntryMetadata) {
        let entry = Entry::new(PathBuf::from("a"), "".to_string(), entry_metadata);

        let dirs = entry.parent_dirs().unwrap();
        assert_eq!(dirs, Vec::<&Path>::new());
    }

    #[rstest]
    fn test_entry_basename(entry_metadata: EntryMetadata) {
        let entry = Entry::new(PathBuf::from("a/b/c"), "".to_string(), entry_metadata);

        let basename = entry.basename().unwrap();
        assert_eq!(basename, "c");
    }
}
