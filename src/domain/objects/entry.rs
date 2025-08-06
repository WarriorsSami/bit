use crate::domain::objects::entry_mode::EntryMode;
use crate::domain::objects::object_id::ObjectId;
use derive_new::new;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, new)]
pub struct Entry {
    pub name: PathBuf,
    pub oid: ObjectId,
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
    use pretty_assertions::assert_eq;
    use rstest::{fixture, rstest};
    use sha1::Digest;

    #[fixture]
    fn oid() -> ObjectId {
        let mut hasher = sha1::Sha1::new();
        hasher.update("test data");
        ObjectId::try_parse(format!("{:x}", hasher.finalize())).unwrap()
    }

    #[rstest]
    fn test_entry_parent_dirs(oid: ObjectId) {
        let entry = Entry::new(PathBuf::from("a/b/c"), oid, EntryMode::Directory);

        let dirs = entry.parent_dirs().unwrap();
        assert_eq!(dirs, vec![Path::new("a"), Path::new("a/b")]);
    }

    #[rstest]
    fn test_entry_parent_dirs_root(oid: ObjectId) {
        let entry = Entry::new(PathBuf::from("a"), oid, EntryMode::Directory);

        let dirs = entry.parent_dirs().unwrap();
        assert_eq!(dirs, Vec::<&Path>::new());
    }

    #[rstest]
    fn test_entry_basename(oid: ObjectId) {
        let entry = Entry::new(PathBuf::from("a/b/c"), oid, EntryMode::Directory);

        let basename = entry.basename().unwrap();
        assert_eq!(basename, "c");
    }
}
