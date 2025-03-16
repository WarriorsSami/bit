use crate::domain::objects::entry::{Entry, EntryMode};
use crate::domain::objects::object::Object;
use crate::domain::objects::object_type::ObjectType;
use anyhow::Context;
use bytes::Bytes;
use std::marker::PhantomData;

#[derive(Debug, Clone)]
pub struct Tree<'tree> {
    entries: Vec<Entry>,
    marker: PhantomData<&'tree ()>,
}

impl<'tree> Tree<'tree> {
    pub fn new(entries: Vec<Entry>) -> Self {
        // sort entries by name
        let mut entries = entries;
        entries.sort_by(|a, b| a.name.cmp(&b.name));

        Self {
            entries,
            marker: Default::default(),
        }
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
                let _object_type: ObjectType = parts
                    .next()
                    .context("Invalid tree object: missing type")?
                    .try_into()?;
                let id = parts.next().context("Invalid tree object: missing id")?;
                let path = parts.next().context("Invalid tree object: missing path")?;

                Ok(Entry::new(path.to_string(), id.to_string(), mode))
            })
            .collect::<anyhow::Result<Vec<Entry>>>()?;

        Ok(Self {
            entries,
            marker: Default::default(),
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
            .map(|tree_entry| {
                format!(
                    "{} {} {}\t{}",
                    tree_entry.mode.as_str(),
                    ObjectType::Blob.as_str(),
                    tree_entry.oid,
                    tree_entry.name
                )
            })
            .collect::<Vec<String>>()
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
            .map(|tree_entry| {
                format!(
                    "{} {} {}\t{}",
                    tree_entry.mode.as_str(),
                    tree_entry.oid,
                    ObjectType::Blob.as_str(),
                    tree_entry.name
                )
            })
            .collect::<Vec<String>>()
            .join("\n")
    }
}
