use crate::domain::objects::object::Object;
use crate::domain::objects::object_type::ObjectType;
use anyhow::Context;
use bytes::Bytes;

#[derive(Debug, Clone)]
pub struct Author {
    name: String,
    email: String,
    timestamp: chrono::DateTime<chrono::Utc>,
}

impl Author {
    pub fn new(name: String, email: String) -> Self {
        Author { name, email, timestamp: chrono::Utc::now() }
    }
    
    pub fn display(&self) -> String {
        format!("{} <{}> {}", self.name, self.email, self.timestamp)
    }
    
    pub fn load_from_env() -> anyhow::Result<Self> {
        let name = std::env::var("GIT_AUTHOR_NAME").context("GIT_AUTHOR_NAME not set")?;
        let email = std::env::var("GIT_AUTHOR_EMAIL").context("GIT_AUTHOR_EMAIL not set")?;
        
        Ok(Self::new(name, email))
    }
}

#[derive(Debug, Clone)]
pub struct Commit {
    tree_oid: String,
    author: Author,
    message: String,
}

impl Commit {
    pub fn new(tree_oid: String, author: Author, message: String) -> Self {
        Commit { tree_oid, author, message }
    }

    pub fn display(&self) -> String {
        format!(
            "tree {}\nauthor {}\n\n{}",
            self.tree_oid,
            self.author.display(),
            self.message
        )
    }
    
    pub fn short_message(&self) -> String {
        self.message.lines().next().unwrap_or("").to_string()
    }
}

impl Object for Commit {
    fn serialize(&self) -> anyhow::Result<Bytes> {
        let object_content = format!(
            "{} {}\0tree {}\nauthor {}\n\n{}",
            self.object_type().as_str(),
            self.display().len(),
            self.tree_oid,
            self.author.display(),
            self.message
        );
        
        Ok(Bytes::from(object_content))
    }

    fn object_type(&self) -> ObjectType {
        ObjectType::Commit
    }
}