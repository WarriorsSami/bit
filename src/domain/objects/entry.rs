use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct Entry {
    pub name: PathBuf,
    pub oid: String,
    pub mode: EntryMode,
}

impl Entry {
    pub fn new(name: PathBuf, oid: String, mode: EntryMode) -> Self {
        Self { name, oid, mode }
    }
    
    pub fn basename(&self) -> anyhow::Result<&str> {
        self.name.file_name()
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

#[derive(Debug, Clone, Default)]
pub enum FileMode {
    #[default]
    Regular,
    Executable,
}

#[derive(Debug, Clone)]
pub enum EntryMode {
    File(FileMode),
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

    #[test]
    fn test_entry_parent_dirs() {
        let entry = Entry::new(PathBuf::from("a/b/c"), "".to_string(), EntryMode::Directory);

        let dirs = entry.parent_dirs().unwrap();
        assert_eq!(dirs, vec![Path::new("a"), Path::new("a/b")]);
    }
    
    #[test]
    fn test_entry_parent_dirs_root() {
        let entry = Entry::new(PathBuf::from("a"), "".to_string(), EntryMode::Directory);

        let dirs = entry.parent_dirs().unwrap();
        assert_eq!(dirs, Vec::<&Path>::new());
    }
    
    #[test]
    fn test_entry_basename() {
        let entry = Entry::new(PathBuf::from("a/b/c"), "".to_string(), EntryMode::Directory);

        let basename = entry.basename().unwrap();
        assert_eq!(basename, "c");
    }
}
