use crate::domain::objects::core::database_entry::DatabaseEntry;
use crate::domain::objects::core::entry_mode::EntryMode;
use crate::domain::objects::core::index_entry::IndexEntry;
use crate::domain::objects::core::object::{Object, Packable};
use crate::domain::objects::core::object_id::ObjectId;
use crate::domain::objects::core::object_type::ObjectType;
use crate::domain::objects::object::Unpackable;
use anyhow::Context;
use bytes::Bytes;
use std::collections::BTreeMap;
use std::io::{BufRead, Write};
use std::marker::PhantomData;
use std::path::Path;

#[derive(Debug, Clone)]
enum TreeEntry<'e> {
    File(IndexEntry),
    Directory(Tree<'e>),
    LazyDirectory(IndexEntry),
}

impl TreeEntry<'_> {
    fn object_type(&self) -> ObjectType {
        match self {
            TreeEntry::File(_) => ObjectType::Blob,
            TreeEntry::Directory(_) | TreeEntry::LazyDirectory(_) => ObjectType::Tree,
        }
    }

    fn mode(&self) -> &EntryMode {
        match self {
            TreeEntry::File(entry) | TreeEntry::LazyDirectory(entry) => &entry.metadata.mode,
            TreeEntry::Directory(_) => &EntryMode::Directory,
        }
    }

    fn oid(&self) -> anyhow::Result<ObjectId> {
        match self {
            TreeEntry::File(entry) | TreeEntry::LazyDirectory(entry) => Ok(entry.oid.clone()),
            TreeEntry::Directory(tree) => tree.object_id(),
        }
    }
}

// TODO: Ponder whether to implement ReadableTree and WritableTree for better separation of concerns
#[derive(Debug, Clone, Default)]
pub struct Tree<'tree> {
    readable_entries: BTreeMap<String, DatabaseEntry>,
    writeable_entries: BTreeMap<String, TreeEntry<'tree>>,
    _marker: PhantomData<&'tree ()>,
}

impl<'tree> Tree<'tree> {
    pub fn build(entries: impl Iterator<Item = &'tree IndexEntry> + 'tree) -> anyhow::Result<Self> {
        let mut root = Self::default();

        for entry in entries {
            let parents = entry.parent_dirs()?;
            root.add_entry(parents, entry)?;
        }

        Ok(root)
    }

    pub fn traverse<F>(&self, func: &F) -> anyhow::Result<()>
    where
        F: Fn(&Tree<'tree>) -> anyhow::Result<()>,
    {
        for entry in &self.writeable_entries {
            if let TreeEntry::Directory(tree) = entry.1 {
                tree.traverse(func)?;
            }
        }
        func(self)?;

        Ok(())
    }

    fn add_entry(&mut self, parents: Vec<&Path>, entry: &IndexEntry) -> anyhow::Result<()> {
        if parents.is_empty() {
            self.writeable_entries.insert(
                entry.basename()?.to_string(),
                TreeEntry::File(entry.clone()),
            );
        } else {
            let parent = parents[0]
                .file_name()
                .and_then(|s| s.to_str())
                .context("Invalid parent")?;
            // TODO: ensure directory names always end with '/'
            let parent = format!("{}/", parent);
            let tree = match self.writeable_entries.get_mut(&parent) {
                Some(TreeEntry::Directory(tree)) => tree,
                _ => {
                    let tree = Self::default();
                    self.writeable_entries
                        .insert(parent.to_string(), TreeEntry::Directory(tree.clone()));

                    match self.writeable_entries.get_mut(&parent) {
                        Some(TreeEntry::Directory(tree)) => tree,
                        _ => unreachable!(),
                    }
                }
            };
            tree.add_entry(parents[1..].to_vec(), entry)?;
        }

        Ok(())
    }

    pub fn entries(&self) -> impl Iterator<Item = (&String, &DatabaseEntry)> {
        self.readable_entries.iter()
    }

    pub fn into_entries(self) -> impl Iterator<Item = (String, DatabaseEntry)> {
        self.readable_entries.into_iter()
    }
}

impl Packable for Tree<'_> {
    fn serialize(&self) -> anyhow::Result<Bytes> {
        let content_bytes: Bytes = self
            .writeable_entries
            .iter()
            .map(|(name, tree_entry)| {
                let mut entry_bytes = Vec::new();
                let name = name.trim_end_matches('/'); // Remove trailing '/' for directories

                let header = format!("{:o} {}", tree_entry.mode().as_u32(), name);
                entry_bytes.write_all(header.as_bytes())?;
                entry_bytes.push(0);
                tree_entry.oid()?.write_h40_to(&mut entry_bytes)?;

                Ok(Bytes::from(entry_bytes))
            })
            .filter_map(|result: anyhow::Result<Bytes>| result.ok())
            .fold(Vec::new(), |mut acc, entry_bytes| {
                acc.extend(entry_bytes);
                acc
            })
            .into();

        let mut tree_bytes = Vec::new();
        let header = format!("{} {}\0", self.object_type().as_str(), content_bytes.len());
        tree_bytes.write_all(header.as_bytes())?;
        tree_bytes.write_all(&content_bytes)?;

        Ok(Bytes::from(tree_bytes))
    }
}

impl Unpackable for Tree<'_> {
    fn deserialize(reader: impl BufRead) -> anyhow::Result<Self> {
        let mut entries = BTreeMap::new();
        let mut reader = reader;

        // Reuse scratch buffers to reduce allocs
        let mut mode_bytes = Vec::new();
        let mut name_bytes = Vec::new();

        loop {
            mode_bytes.clear();
            // Read "mode " (space-delimited)
            let n = reader.read_until(b' ', &mut mode_bytes)?;
            if n == 0 {
                break; // clean EOF: no more entries
            }
            // Must end with ' ' or it's malformed
            if *mode_bytes.last().unwrap() != b' ' {
                return Err(anyhow::anyhow!("unexpected EOF in mode"));
            }
            mode_bytes.pop(); // drop the space

            let mode_str = std::str::from_utf8(&mode_bytes)?;
            let mode = EntryMode::from_octal_str(mode_str)?; // parse without extra String

            // Read "name\0"
            name_bytes.clear();
            let n = reader.read_until(b'\0', &mut name_bytes)?;
            if n == 0 || *name_bytes.last().unwrap() != b'\0' {
                return Err(anyhow::anyhow!("unexpected EOF in name"));
            }
            name_bytes.pop(); // drop NUL
            let name = std::str::from_utf8(&name_bytes)?.to_owned();

            // Read object id
            let oid =
                ObjectId::read_h40_from(&mut reader).context("unexpected EOF in object id")?;

            entries.insert(name, DatabaseEntry::new(oid, mode));
        }

        Ok(Tree {
            readable_entries: entries,
            writeable_entries: Default::default(),
            _marker: Default::default(),
        })
    }
}

impl Object for Tree<'_> {
    fn object_type(&self) -> ObjectType {
        ObjectType::Tree
    }

    fn display(&self) -> String {
        self.writeable_entries
            .iter()
            .map(|(name, tree_entry)| {
                let name = name.trim_end_matches('/'); // Remove trailing '/' for directories

                format!(
                    "{} {} {}\t{}",
                    tree_entry.mode().as_str(),
                    tree_entry.object_type().as_str(),
                    tree_entry.oid().unwrap_or_default().as_ref(),
                    name
                )
            })
            .collect::<Vec<String>>()
            .join("\n")
    }
}
