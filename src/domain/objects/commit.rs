use crate::domain::objects::object::{Object, Packable};
use crate::domain::objects::object_id::ObjectId;
use crate::domain::objects::object_type::ObjectType;
use anyhow::Context;
use bytes::Bytes;
use std::io::Write;

#[derive(Debug, Clone)]
pub struct Author {
    name: String,
    email: String,
    timestamp: chrono::DateTime<chrono::Local>,
}

impl Author {
    pub fn new(name: String, email: String) -> Self {
        Author {
            name,
            email,
            timestamp: chrono::Local::now(),
        }
    }

    pub fn display(&self) -> String {
        format!(
            "{} <{}> {} {}",
            self.name,
            self.email,
            self.timestamp.timestamp(),
            self.timestamp.format("%z")
        )
    }

    pub fn load_from_env() -> anyhow::Result<Self> {
        let name = std::env::var("GIT_AUTHOR_NAME").context("GIT_AUTHOR_NAME not set")?;
        let email = std::env::var("GIT_AUTHOR_EMAIL").context("GIT_AUTHOR_EMAIL not set")?;

        Ok(Self::new(name, email))
    }
}

#[derive(Debug, Clone)]
pub struct Commit<'commit> {
    parent: Option<&'commit str>,
    tree_oid: ObjectId,
    author: Author,
    committer: Author,
    message: String,
}

impl<'commit> Commit<'commit> {
    pub fn new(
        parent: Option<&'commit str>,
        tree_oid: ObjectId,
        author: Author,
        message: String,
    ) -> Self {
        Commit {
            parent,
            tree_oid,
            author: author.clone(),
            committer: author,
            message,
        }
    }

    pub fn short_message(&self) -> String {
        self.message.lines().next().unwrap_or("").to_string()
    }
}

// TODO: Convert from Bytes instead of &str
impl<'commit> TryFrom<&'commit str> for Commit<'commit> {
    type Error = anyhow::Error;

    fn try_from(data: &'commit str) -> anyhow::Result<Self> {
        let mut lines = data.lines();
        lines
            .next()
            .context("Invalid commit object: missing header")?;

        let tree_oid = lines
            .next()
            .context("Invalid commit object: missing tree")?
            .split_whitespace()
            .nth(1)
            .context("Invalid commit object: missing tree")?;
        let tree_oid = ObjectId::try_parse(tree_oid.to_string())
            .context("Invalid commit object: invalid tree OID")?;

        let parent = lines
            .next()
            .filter(|line| line.starts_with("parent"))
            .map(|line| {
                line.split_whitespace()
                    .nth(1)
                    .context("Invalid commit object: missing parent")
            })
            .transpose()?;

        let author_line = lines
            .next()
            .context("Invalid commit object: missing author")?;
        let author_name = author_line
            .split_whitespace()
            .skip(1)
            .take(1)
            .collect::<Vec<&str>>()
            .join(" ");
        let author_email = author_line
            .split_whitespace()
            .skip(2)
            .take(1)
            .collect::<Vec<&str>>()
            .join(" ")
            .trim_matches(|c| c == '<' || c == '>')
            .to_string();
        let author = Author::new(author_name, author_email);

        let message = lines.collect::<Vec<&str>>().join("\n");

        Ok(Self::new(parent, tree_oid, author, message))
    }
}

impl Packable for Commit<'_> {
    fn serialize(&self) -> anyhow::Result<Bytes> {
        let mut object_content = vec![];

        // object_content.push(format!(
        //     "{} {}\0",
        //     self.object_type().as_str(),
        //     self.display().len()
        // ));
        object_content.push(format!("tree {}", self.tree_oid.as_ref()));
        if let Some(parent) = &self.parent {
            object_content.push(format!("parent {parent}"));
        }
        object_content.push(format!("author {}", self.author.display()));
        object_content.push(format!("committer {}", self.committer.display()));
        object_content.push(String::new());
        object_content.push(self.message.to_string());

        let object_content = object_content.join("\n");

        let mut content_bytes = Vec::new();
        content_bytes.write_all(object_content.as_bytes())?;

        let mut commit_bytes = Vec::new();
        let header = format!("{} {}\0", self.object_type().as_str(), content_bytes.len());
        commit_bytes.write_all(header.as_bytes())?;
        commit_bytes.write_all(&content_bytes)?;

        Ok(Bytes::from(commit_bytes))
    }
}

impl Object for Commit<'_> {
    fn object_type(&self) -> ObjectType {
        ObjectType::Commit
    }

    fn display(&self) -> String {
        let mut lines = vec![];

        lines.push(format!("tree {}", self.tree_oid.as_ref()));
        if let Some(parent) = &self.parent {
            lines.push(format!("parent {parent}"));
        }
        lines.push(format!("author {}", self.author.display()));
        lines.push(format!("committer {}", self.committer.display()));
        lines.push(String::new());
        lines.push(self.message.to_string());

        lines.join("\n")
    }
}
