use crate::domain::objects::entry::{Entry, EntryMetadata, EntryMode};
use crate::domain::objects::object::Object;
use crate::domain::objects::object_type::ObjectType;
use anyhow::Context;
use bytes::Bytes;
use std::collections::BTreeMap;
use std::marker::PhantomData;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
enum TreeEntry<'e> {
    File(Entry),
    Directory(Tree<'e>),
    LazyDirectory(Entry),
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

    fn oid(&self) -> anyhow::Result<String> {
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
    pub fn build(entries: Vec<Entry>) -> anyhow::Result<Self> {
        let mut entries = entries;
        entries.sort_by(|a, b| a.name.cmp(&b.name));

        let mut root = Self::default();

        for entry in entries {
            let parents = entry.parent_dirs()?;
            root.add_entry(parents, &entry)?;
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

    fn add_entry(&mut self, parents: Vec<&Path>, entry: &Entry) -> anyhow::Result<()> {
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
            let tree = match self.entries.get_mut(parent) {
                Some(TreeEntry::Directory(tree)) => tree,
                _ => {
                    let tree = Self::default();
                    self.entries
                        .insert(parent.to_string(), TreeEntry::Directory(tree.clone()));

                    match self.entries.get_mut(parent) {
                        Some(TreeEntry::Directory(tree)) => tree,
                        _ => unreachable!(),
                    }
                }
            };
            tree.add_entry(parents[1..].to_vec(), entry)?;
        }

        Ok(())
    }

    fn from(data: &'tree str) -> anyhow::Result<Self> {
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
                let id = parts.next().context("Invalid tree object: missing id")?;
                let path = parts.next().context("Invalid tree object: missing path")?;
                let metadata = EntryMetadata {
                    mode,
                    ..Default::default()
                };

                Ok((
                    path.to_string(),
                    match object_type {
                        ObjectType::Blob => TreeEntry::File(Entry {
                            name: PathBuf::from(path),
                            oid: id.to_string(),
                            metadata,
                        }),
                        ObjectType::Tree => TreeEntry::LazyDirectory(Entry {
                            name: PathBuf::from(path),
                            oid: id.to_string(),
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

impl<'tree> TryFrom<&'tree str> for Tree<'tree> {
    type Error = anyhow::Error;

    fn try_from(data: &'tree str) -> anyhow::Result<Self> {
        Tree::from(data)
    }
}

impl Object for Tree<'_> {
    fn serialize(&self) -> anyhow::Result<Bytes> {
        let entries = self
            .entries
            .iter()
            .map(|(name, tree_entry)| {
                Ok(format!(
                    "{} {} {}\t{}",
                    tree_entry.mode().as_str(),
                    tree_entry.object_type().as_str(),
                    tree_entry.oid()?.as_str(),
                    name
                ))
            })
            .collect::<anyhow::Result<Vec<String>>>()?
            .join("\n");

        let object_content = format!(
            "{} {}\0{}",
            self.object_type().as_str(),
            entries.len(),
            entries
        );

        Ok(Bytes::from(object_content))
    }

    fn object_type(&self) -> ObjectType {
        ObjectType::Tree
    }

    fn display(&self) -> String {
        self.entries
            .iter()
            .map(|(name, tree_entry)| {
                format!(
                    "{} {} {}\t{}",
                    tree_entry.mode().as_str(),
                    tree_entry.object_type().as_str(),
                    tree_entry.oid().unwrap_or_default(),
                    name
                )
            })
            .collect::<Vec<String>>()
            .join("\n")
    }
}
