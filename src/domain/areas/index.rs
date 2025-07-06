use crate::domain::objects::entry::Entry;
use std::collections::BTreeSet;
use std::path::Path;

#[derive(Debug, Clone)]
pub struct Index {
    path: Box<Path>,
    entries: BTreeSet<Entry>,
}

impl Index {
    pub fn new(path: Box<Path>) -> Self {
        Index {
            path,
            entries: BTreeSet::new(),
        }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn add(&mut self, entry: Entry) -> anyhow::Result<()> {
        self.entries.insert(entry);
        Ok(())
    }

    pub fn write_updates(&self) -> anyhow::Result<()> {
        todo!()
    }
}
