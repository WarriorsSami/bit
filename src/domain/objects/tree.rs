use crate::domain::objects::core::entry_mode::EntryMode;
use crate::domain::objects::core::index_entry::{EntryMetadata, IndexEntry};
use crate::domain::objects::core::object::{Object, Packable};
use crate::domain::objects::core::object_id::ObjectId;
use crate::domain::objects::core::object_type::ObjectType;
use anyhow::Context;
use bytes::Bytes;
use std::collections::BTreeMap;
use std::io::Write;
use std::marker::PhantomData;
use std::path::{Path, PathBuf};

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

#[derive(Debug, Clone, Default)]
pub struct Tree<'tree> {
    entries: BTreeMap<String, TreeEntry<'tree>>,
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
        for entry in &self.entries {
            if let TreeEntry::Directory(tree) = entry.1 {
                tree.traverse(func)?;
            }
        }
        func(self)?;

        Ok(())
    }

    fn add_entry(&mut self, parents: Vec<&Path>, entry: &IndexEntry) -> anyhow::Result<()> {
        if parents.is_empty() {
            self.entries.insert(
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
            let tree = match self.entries.get_mut(&parent) {
                Some(TreeEntry::Directory(tree)) => tree,
                _ => {
                    let tree = Self::default();
                    self.entries
                        .insert(parent.to_string(), TreeEntry::Directory(tree.clone()));

                    match self.entries.get_mut(&parent) {
                        Some(TreeEntry::Directory(tree)) => tree,
                        _ => unreachable!(),
                    }
                }
            };
            tree.add_entry(parents[1..].to_vec(), entry)?;
        }

        Ok(())
    }
}

// TODO: Convert from Bytes instead of &str
impl<'tree> TryFrom<&'tree str> for Tree<'tree> {
    type Error = anyhow::Error;

    fn try_from(data: &'tree str) -> anyhow::Result<Self> {
        let entries = data
            .split("\0")
            .nth(1)
            .context("Invalid tree object: missing entries")?
            .split("\n")
            .filter(|line| !line.is_empty())
            .map(|line| {
                let mut parts = line.split_whitespace();
                let mode: EntryMode = parts
                    .next()
                    .context("Invalid tree object: missing mode")?
                    .try_into()?;
                let object_type: ObjectType = parts
                    .next()
                    .context("Invalid tree object: missing type")?
                    .try_into()?;
                let oid = ObjectId::try_parse(String::from(
                    parts.next().context("Invalid tree object: missing id")?,
                ))?;
                let path = parts.next().context("Invalid tree object: missing path")?;
                let metadata = EntryMetadata {
                    mode,
                    ..Default::default()
                };

                Ok((
                    path.to_string(),
                    match object_type {
                        ObjectType::Blob => TreeEntry::File(IndexEntry {
                            name: PathBuf::from(path),
                            oid,
                            metadata,
                        }),
                        ObjectType::Tree => TreeEntry::LazyDirectory(IndexEntry {
                            name: PathBuf::from(path),
                            oid,
                            metadata,
                        }),
                        _ => unreachable!(),
                    },
                ))
            })
            .collect::<anyhow::Result<BTreeMap<String, TreeEntry<'tree>>>>()?;

        Ok(Self {
            entries,
            _marker: Default::default(),
        })
    }
}

impl Packable for Tree<'_> {
    fn serialize(&self) -> anyhow::Result<Bytes> {
        let content_bytes: Bytes = self
            .entries
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

impl Object for Tree<'_> {
    fn object_type(&self) -> ObjectType {
        ObjectType::Tree
    }

    fn display(&self) -> String {
        self.entries
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
