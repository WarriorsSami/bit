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
        Author {
            name,
            email,
            timestamp: chrono::Utc::now(),
        }
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
    parent: Option<String>,
    tree_oid: String,
    author: Author,
    committer: Author,
    message: String,
}

impl Commit {
    pub fn new(parent: Option<String>, tree_oid: String, author: Author, message: String) -> Self {
        Commit {
            parent,
            tree_oid,
            author: author.clone(),
            committer: author,
            message,
        }
    }

    pub fn display(&self) -> String {
        let mut lines = vec![];

        lines.push(format!("tree {}", self.tree_oid));
        if let Some(parent) = &self.parent {
            lines.push(format!("parent {}", parent));
        }
        lines.push(format!("author {}", self.author.display()));
        lines.push(format!("committer {}", self.committer.display()));
        lines.push(String::new());
        lines.push(self.message.clone());

        lines.join("\n")
    }

    pub fn short_message(&self) -> String {
        self.message.lines().next().unwrap_or("").to_string()
    }
}

impl Object for Commit {
    fn serialize(&self) -> anyhow::Result<Bytes> {
        let mut object_content = vec![];

        object_content.push(format!(
            "{} {}\0",
            self.object_type().as_str(),
            self.display().len()
        ));
        object_content.push(format!("tree {}", self.tree_oid));
        if let Some(parent) = &self.parent {
            object_content.push(format!("parent {}", parent));
        }
        object_content.push(format!("author {}", self.author.display()));
        object_content.push(format!("committer {}", self.committer.display()));
        object_content.push(String::new());
        object_content.push(self.message.clone());

        let object_content = object_content.join("\n");

        Ok(Bytes::from(object_content))
    }

    fn object_type(&self) -> ObjectType {
        ObjectType::Commit
    }
}
