use crate::domain::objects::object::Object;
use crate::domain::objects::object_type::ObjectType;
use bytes::Bytes;

pub type TreeEntry = (String, String);

const MODE: &str = "100644";

#[derive(Debug, Clone)]
pub struct Tree {
    entries: Vec<TreeEntry>,
}

impl Tree {
    pub fn new(entries: Vec<TreeEntry>) -> Self {
        Self { entries }
    }

    pub fn display(&self) -> String {
        self.entries
            .iter()
            .map(|(path, id)| format!("{} {} {}\t{}", MODE, id, ObjectType::Blob.as_str(), path))
            .collect::<Vec<String>>()
            .join("\n")
    }
}

impl Object for Tree {
    fn serialize(&self) -> anyhow::Result<Bytes> {
        let entries = self
            .entries
            .iter()
            .map(|(path, id)| format!("{} {} {}\0{}", MODE, ObjectType::Blob.as_str(), id, path))
            .collect::<Vec<String>>()
            .join("");
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
}
