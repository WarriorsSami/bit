use std::marker::PhantomData;
use crate::domain::objects::object::Object;
use crate::domain::objects::object_type::ObjectType;
use anyhow::Context;
use bytes::Bytes;

#[derive(Debug, Clone)]
pub struct TreeEntry {
    pub path: String,
    pub id: String,
}

impl TreeEntry {
    pub fn new(path: String, id: String) -> Self {
        Self { path, id }
    }
}

const MODE: &str = "100644";

#[derive(Debug, Clone)]
pub struct Tree<'tree> {
    entries: Vec<TreeEntry>,
    marker: PhantomData<&'tree ()>,
}

impl<'tree> Tree<'tree> {
    // TODO: sort entries
    pub fn new(entries: Vec<TreeEntry>) -> Self {
        Self { entries, marker: Default::default() }
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
                let _mode = parts.next().context("Invalid tree object: missing mode")?;
                let _type = parts.next().context("Invalid tree object: missing type")?;
                let id = parts
                    .next()
                    .context("Invalid tree object: missing id")?;
                let path = parts
                    .next()
                    .context("Invalid tree object: missing path")?;
                
                Ok(TreeEntry::new(path.to_string(), id.to_string()))
            })
            .collect::<anyhow::Result<Vec<TreeEntry>>>()?;

        Ok(Self { entries, marker: Default::default() })
    }
}

impl<'tree> TryFrom<&'tree str> for Tree<'tree> {
    type Error = anyhow::Error;

    fn try_from(data: &'tree str) -> anyhow::Result<Self> {
        Tree::from(data)
    }
}

impl<'tree> Object for Tree<'_> {
    fn serialize(&self) -> anyhow::Result<Bytes> {
        let entries = self
            .entries
            .iter()
            .map(|tree_entry| {
                format!(
                    "{} {} {}\t{}",
                    MODE,
                    ObjectType::Blob.as_str(),
                    tree_entry.id,
                    tree_entry.path
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
                    MODE,
                    tree_entry.id,
                    ObjectType::Blob.as_str(),
                    tree_entry.path
                )
            })
            .collect::<Vec<String>>()
            .join("\n")
    }
}
