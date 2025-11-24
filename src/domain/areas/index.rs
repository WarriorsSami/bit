use crate::domain::objects::checksum::Checksum;
use crate::domain::objects::index_entry::{ENTRY_BLOCK, ENTRY_MIN_SIZE, EntryMetadata, IndexEntry};
use crate::domain::objects::index_header::IndexHeader;
use crate::domain::objects::object::{Packable, Unpackable};
use crate::domain::objects::{HEADER_SIZE, SIGNATURE, VERSION};
use anyhow::anyhow;
use bytes::Bytes;
use std::collections::{BTreeMap, BTreeSet};
use std::ops::DerefMut;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct Index {
    path: Box<Path>,
    entries: BTreeMap<Box<Path>, IndexEntry>,
    children: BTreeMap<Box<Path>, BTreeSet<Box<Path>>>,
    header: IndexHeader,
    changed: bool,
}

impl Index {
    pub fn new(path: Box<Path>) -> Self {
        Index {
            path,
            entries: BTreeMap::new(),
            children: BTreeMap::new(),
            header: IndexHeader::new(String::from(SIGNATURE), VERSION, 0),
            changed: false,
        }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn entry_by_path(&self, path: &Path) -> Option<&IndexEntry> {
        self.entries.get(path)
    }

    fn clear(&mut self) {
        self.entries.clear();
        self.children.clear();
        self.header = IndexHeader::empty();
        self.changed = false;
    }

    pub fn rehydrate(&mut self) -> anyhow::Result<()> {
        if !self.path().exists() {
            self.clear();
            // create the index file
            std::fs::File::create(self.path())?;
        }

        let mut index_file = std::fs::OpenOptions::new().read(true).open(self.path())?;
        let mut lock = file_guard::lock(&mut index_file, file_guard::Lock::Shared, 0, 1)?;

        self.clear();

        // if the index file is empty, return early
        if lock.deref_mut().metadata()?.len() == 0 {
            return Ok(());
        }

        let mut reader = Checksum::new(lock);
        let entries_count = self.parse_header(&mut reader)?;
        self.parse_entries(entries_count, &mut reader)?;

        reader.verify()
    }

    pub fn is_directly_tracked(&self, path: &Path) -> bool {
        self.entries.contains_key(path) || self.children.contains_key(path)
    }

    fn parse_header(&self, reader: &mut Checksum) -> anyhow::Result<u32> {
        let header_bytes = reader.read(HEADER_SIZE)?;
        let header_reader = std::io::Cursor::new(header_bytes.clone());
        let header = IndexHeader::deserialize(header_reader)?;

        if header.marker != SIGNATURE {
            return Err(anyhow!("Invalid index file signature"));
        }

        if header.version != VERSION {
            return Err(anyhow!(
                "Unsupported index file version: {}",
                header.version
            ));
        }

        Ok(header.entries_count)
    }

    fn parse_entries(&mut self, entries_count: u32, reader: &mut Checksum) -> anyhow::Result<()> {
        for _ in 0..entries_count {
            let entry_bytes = reader.read(ENTRY_MIN_SIZE)?;
            let mut entry_bytes = entry_bytes.to_vec();

            while entry_bytes[entry_bytes.len() - 1] != 0 {
                entry_bytes = [entry_bytes, reader.read(ENTRY_BLOCK)?.to_vec()].concat();
            }

            let entry_bytes = Bytes::from(entry_bytes);
            let entry_reader = std::io::Cursor::new(entry_bytes.clone());
            let entry = IndexEntry::deserialize(entry_reader)?;

            self.store_entry(&entry)?;
        }

        self.header.entries_count = entries_count;

        Ok(())
    }

    fn discard_conflicts(&mut self, entry: &IndexEntry) -> anyhow::Result<()> {
        entry
            .parent_dirs()?
            .into_iter()
            .map(|parent| self.remove_entry(parent))
            .collect::<Result<Vec<_>, _>>()?;
        self.remove_children(&entry.name)
    }

    fn store_entry(&mut self, entry: &IndexEntry) -> anyhow::Result<()> {
        let entry_parents = entry
            .parent_dirs()?
            .into_iter()
            .map(|parent| parent.to_owned().into_boxed_path())
            .collect::<BTreeSet<_>>();

        self.entries
            .insert(entry.name.clone().into_boxed_path(), entry.clone());

        for parent in entry_parents {
            self.children
                .entry(parent.clone())
                .or_default()
                .insert(entry.name.clone().into_boxed_path());
        }

        Ok(())
    }

    fn remove_children(&mut self, path_name: &Path) -> anyhow::Result<()> {
        if let Some(children) = self.children.remove(path_name) {
            for child in children {
                self.remove_entry(&child)?;
            }
        }

        Ok(())
    }

    fn remove_entry(&mut self, path_name: &Path) -> anyhow::Result<()> {
        match self.entries.remove(path_name) {
            None => Ok(()),
            Some(entry) => {
                entry
                    .parent_dirs()?
                    .into_iter()
                    .map(|parent| parent.to_owned().into_boxed_path())
                    .for_each(|parent| {
                        if let Some(children) = self.children.get_mut(&parent) {
                            children.remove(path_name);
                            if children.is_empty() {
                                self.children.remove(&parent);
                            }
                        }
                    });

                Ok(())
            }
        }
    }

    pub fn add(&mut self, entry: IndexEntry) -> anyhow::Result<()> {
        self.discard_conflicts(&entry)?;
        self.store_entry(&entry)?;

        self.header.entries_count = self.entries.len() as u32;
        self.changed = true;

        Ok(())
    }

    pub fn remove(&mut self, path: PathBuf) -> anyhow::Result<()> {
        self.remove_entry(&path)?;
        self.remove_children(&path)?;

        self.header.entries_count = self.entries.len() as u32;
        self.changed = true;

        Ok(())
    }

    pub fn write_updates(&mut self) -> anyhow::Result<()> {
        let mut index_file = std::fs::OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(self.path())?;
        let lock = file_guard::lock(&mut index_file, file_guard::Lock::Exclusive, 0, 1)?;

        let mut writer = Checksum::new(lock);

        self.header = IndexHeader {
            entries_count: self.entries.len() as u32,
            ..self.header.clone()
        };
        let header_bytes = self.header.serialize()?;
        writer.write(&header_bytes)?;

        for entry in self.entries() {
            let entry_bytes = entry.serialize()?;
            writer.write(&entry_bytes)?;
        }

        writer.write_checksum()?;
        self.changed = false;

        Ok(())
    }

    pub fn update_entry_stat(&mut self, entry: &IndexEntry, stat: EntryMetadata) {
        let entry_key = entry.name.clone().into_boxed_path();
        if let Some(existing_entry) = self.entries.get_mut(&entry_key) {
            existing_entry.metadata = stat;
            self.changed = true;
        }
    }

    pub fn entries(&self) -> impl Iterator<Item = &IndexEntry> {
        self.entries.values()
    }

    pub fn into_entries(self) -> impl Iterator<Item = IndexEntry> {
        self.entries.into_values()
    }

    pub fn entries_under_path(&self, path: &Path) -> Vec<PathBuf> {
        self.entries
            .keys()
            .filter(|entry_path| {
                // If path is ".", include all entries
                if path == Path::new(".") {
                    return true;
                }
                // Otherwise, check if the entry is under the given path
                entry_path.starts_with(path) || entry_path.as_ref() == path
            })
            .map(|p| p.to_path_buf())
            .collect()
    }
}
