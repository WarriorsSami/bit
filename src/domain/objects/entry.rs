use std::path::{Path, PathBuf};
use derive_new::new;
use crate::domain::objects::entry_mode::EntryMode;

#[derive(Debug, Clone, new)]
pub struct Entry {
    pub name: PathBuf,
    pub oid: String,
    pub mode: EntryMode,
}

impl Entry {
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
