use crate::domain::objects::index_entry::{ENTRY_BLOCK, ENTRY_MIN_SIZE, IndexEntry};
use crate::domain::objects::object::{Packable, Unpackable};
use anyhow::anyhow;
use byteorder::{ByteOrder, WriteBytesExt};
use bytes::Bytes;
use derive_new::new;
use file_guard::FileGuard;
use sha1::{Digest, Sha1};
use std::collections::{BTreeMap, BTreeSet};
use std::io::{Read, Write};
use std::ops::DerefMut;
use std::path::Path;

const CHECKSUM_SIZE: usize = 20; // SHA1 produces a 20-byte hash
const HEADER_SIZE: usize = 12; // 4 bytes for marker, 4 for version, 4 for entries_count
const SIGNATURE: &str = "DIRC"; // Signature for the index file
const VERSION: u32 = 2; // Version of the index file format

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

impl Unpackable for Header {
    fn deserialize(bytes: Bytes) -> anyhow::Result<Self> {
        if bytes.len() < HEADER_SIZE {
            return Err(anyhow!("Invalid header size"));
        }

        let marker = String::from_utf8(bytes[0..4].to_vec())
            .map_err(|_| anyhow!("Invalid marker in index header"))?;
        let version = byteorder::NetworkEndian::read_u32(&bytes[4..8]);
        let entries_count = byteorder::NetworkEndian::read_u32(&bytes[8..12]);

        Ok(Header {
            marker,
            version,
            entries_count,
        })
    }
}

#[derive(Debug)]
struct Checksum<'f> {
    file: FileGuard<&'f mut std::fs::File>,
    digest: Sha1,
}

impl<'f> Checksum<'f> {
    fn new(file: FileGuard<&'f mut std::fs::File>) -> Self {
        Checksum {
            file,
            digest: Sha1::new(),
        }
    }

    fn read(&mut self, size: usize) -> anyhow::Result<Bytes> {
        let mut buffer = vec![0; size];
        self.file
            .deref_mut()
            .read_exact(&mut buffer)
            .map_err(|_| anyhow!("Unexpected end-of-file while reading index"))?;

        self.digest.update(&buffer);
        Ok(Bytes::from(buffer))
    }

    fn write(&mut self, data: &[u8]) -> anyhow::Result<()> {
        self.file.deref_mut().write_all(data)?;
        self.digest.update(data);
        Ok(())
    }

    fn write_checksum(&mut self) -> anyhow::Result<()> {
        let checksum = self.digest.clone().finalize();
        self.file
            .deref_mut()
            .write_all(checksum.as_slice())
            .map_err(|_| anyhow!("Failed to write checksum to index file"))?;

        Ok(())
    }

    fn verify(&mut self) -> anyhow::Result<()> {
        let mut expected_checksum = [0u8; CHECKSUM_SIZE];
        self.file.deref_mut().read_exact(&mut expected_checksum)?;

        let actual_checksum = self.digest.clone().finalize();
        let actual_checksum = actual_checksum.as_slice();

        if expected_checksum != actual_checksum {
            return Err(anyhow!("Checksum does not match value stored on disk"));
        }

        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct Index {
    path: Box<Path>,
    entries: BTreeMap<Box<Path>, IndexEntry>,
    children: BTreeMap<Box<Path>, BTreeSet<Box<Path>>>,
    header: Header,
    changed: bool,
}

impl Index {
    pub fn new(path: Box<Path>) -> Self {
        Index {
            path,
            entries: BTreeMap::new(),
            children: BTreeMap::new(),
            header: Header::new(String::from(SIGNATURE), VERSION, 0),
            changed: false,
        }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    fn clear(&mut self) {
        self.entries.clear();
        self.children.clear();
        self.header = Header {
            entries_count: 0,
            ..self.header.clone()
        };
        self.changed = false;
    }

    pub fn rehydrate(&mut self) -> anyhow::Result<()> {
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

    fn parse_header(&self, reader: &mut Checksum) -> anyhow::Result<u32> {
        let header_bytes = reader.read(HEADER_SIZE)?;
        let header = Header::deserialize(header_bytes)?;

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
            let entry = IndexEntry::deserialize(entry_bytes)?;

            self.entries
                .insert(entry.name.clone().into_boxed_path(), entry);
        }

        self.header.entries_count = entries_count;

        Ok(())
    }

    // TODO: Rollback on error
    fn discard_conflicts(&mut self, entry: &IndexEntry) -> anyhow::Result<()> {
        entry.parent_dirs()?.into_iter().for_each(|parent| {
            let _ = self.remove_entry(parent);
        });
        self.remove_children(&entry.name)
    }

    fn store_entry(&mut self, entry: IndexEntry) -> anyhow::Result<()> {
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

        self.store_entry(entry)?;

        self.header.entries_count = self.entries.len() as u32;
        self.changed = true;

        Ok(())
    }

    // TODO: Ponder whether still needing to acquire the write lock here
    pub fn write_updates(&mut self) -> anyhow::Result<()> {
        let mut index_file = std::fs::OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(self.path())?;
        let lock = file_guard::lock(&mut index_file, file_guard::Lock::Exclusive, 0, 1)?;

        let mut writer = Checksum::new(lock);

        self.header = Header {
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

    pub fn entries(&self) -> impl Iterator<Item = &IndexEntry> {
        self.entries.values()
    }

    pub fn into_entries(self) -> impl Iterator<Item = IndexEntry> {
        self.entries.into_values()
    }
}
