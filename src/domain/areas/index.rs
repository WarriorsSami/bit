use crate::domain::objects::index_entry::IndexEntry;
use crate::domain::objects::object::Packable;
use byteorder::WriteBytesExt;
use bytes::Bytes;
use derive_new::new;
use sha1::{Digest, Sha1};
use std::collections::BTreeSet;
use std::io::Write;
use std::path::Path;

#[derive(Debug, Clone, new)]
struct Header {
    marker: String,
    version: u32,
    entries_count: u32,
}

impl Packable for Header {
    fn serialize(&self) -> anyhow::Result<Bytes> {
        // pack!(self.marker, self.version, self.entries_count => "a4N2")
        let mut bytes = Vec::new();
        bytes.write_all(self.marker.as_bytes())?;
        bytes.write_u32::<byteorder::NetworkEndian>(self.version)?;
        bytes.write_u32::<byteorder::NetworkEndian>(self.entries_count)?;

        Ok(Bytes::from(bytes))
    }
}

#[derive(Debug, Clone)]
pub struct Index {
    path: Box<Path>,
    entries: BTreeSet<IndexEntry>,
    header: Header,
}

impl Index {
    pub fn new(path: Box<Path>) -> Self {
        Index {
            path,
            entries: BTreeSet::new(),
            header: Header::new(String::from("DIRC"), 2, 0),
        }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn add(&mut self, entry: IndexEntry) -> anyhow::Result<()> {
        self.entries.insert(entry);
        Ok(())
    }

    pub fn write_updates(&mut self) -> anyhow::Result<()> {
        let mut index_file = std::fs::OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(self.path())?;
        let mut lock = file_guard::lock(&mut index_file, file_guard::Lock::Exclusive, 0, 1)?;
        let mut digest = Sha1::new();

        let mut write = |data: &[u8]| -> anyhow::Result<()> {
            lock.write_all(data)?;
            digest.update(data);

            Ok(())
        };

        self.header = Header {
            entries_count: self.entries.len() as u32,
            ..self.header.clone()
        };
        let header_bytes = self.header.serialize()?;
        write(&header_bytes)?;

        for entry in &self.entries {
            let entry_bytes = entry.serialize()?;
            write(&entry_bytes)?;
        }

        lock.write_all(&digest.finalize())?;

        Ok(())
    }
}
