use crate::artifacts::objects::object::Unpackable;
use crate::artifacts::objects::object::{Object, Packable};
use crate::artifacts::objects::object_id::ObjectId;
use crate::artifacts::objects::object_type::ObjectType;
use anyhow::Context;
use bytes::Bytes;
use std::io::{BufRead, Write};

#[derive(Debug, Clone)]
pub struct Author {
    name: String,
    email: String,
    timestamp: chrono::DateTime<chrono::FixedOffset>,
}

impl Author {
    pub fn new(name: String, email: String) -> Self {
        Author {
            name,
            email,
            timestamp: chrono::Local::now().fixed_offset(),
        }
    }

    pub fn new_with_timestamp(
        name: String,
        email: String,
        timestamp: chrono::DateTime<chrono::FixedOffset>,
    ) -> Self {
        Author {
            name,
            email,
            timestamp,
        }
    }

    pub fn display_name(&self) -> String {
        format!("{} <{}>", self.name, self.email)
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
        let timestamp = std::env::var("GIT_AUTHOR_DATE").ok().and_then(|date_str| {
            chrono::DateTime::parse_from_rfc2822(&date_str)
                .or_else(|_| chrono::DateTime::parse_from_str(&date_str, "%Y-%m-%d %H:%M:%S %z"))
                .ok()
        });

        match timestamp {
            Some(ts) => Ok(Author::new_with_timestamp(name, email, ts)),
            None => Ok(Author::new(name, email)),
        }
    }

    pub fn readable_timestamp(&self) -> String {
        self.timestamp
            .format("%a %b %-d %H:%M:%S %Y %z")
            .to_string()
    }
}

impl TryFrom<&str> for Author {
    type Error = anyhow::Error;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        // Format: "name <email> timestamp timezone"
        // Split from right to get timezone and timestamp first
        let parts: Vec<&str> = value.rsplitn(3, ' ').collect();
        if parts.len() < 3 {
            return Err(anyhow::anyhow!("Invalid author format"));
        }

        let timezone = parts[0];
        let timestamp = parts[1]
            .parse::<i64>()
            .map_err(|_| anyhow::anyhow!("Invalid timestamp"))?;
        let name_email_part = parts[2]; // "name <email>"

        // Extract email from within angle brackets
        let email_start = name_email_part
            .find('<')
            .ok_or_else(|| anyhow::anyhow!("Invalid author format: missing '<'"))?;
        let email_end = name_email_part
            .find('>')
            .ok_or_else(|| anyhow::anyhow!("Invalid author format: missing '>'"))?;

        let name = name_email_part[..email_start].trim().to_string();
        let email = name_email_part[email_start + 1..email_end].to_string();

        let datetime = chrono::DateTime::from_timestamp(timestamp, 0)
            .ok_or_else(|| anyhow::anyhow!("Invalid timestamp"))?;
        let datetime = chrono::DateTime::parse_from_str(
            &format!("{} {}", datetime.format("%Y-%m-%d %H:%M:%S"), timezone),
            "%Y-%m-%d %H:%M:%S %z",
        )
        .map_err(|_| anyhow::anyhow!("Invalid timezone"))?;

        Ok(Author {
            name,
            email,
            timestamp: datetime,
        })
    }
}

#[derive(Debug, Clone)]
pub struct Commit {
    parent: Option<ObjectId>,
    tree_oid: ObjectId,
    author: Author,
    committer: Author,
    message: String,
}

impl Commit {
    pub fn new(
        parent: Option<ObjectId>,
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

    pub fn message(&self) -> &str {
        &self.message
    }

    pub fn tree_oid(&self) -> &ObjectId {
        &self.tree_oid
    }

    pub fn parent(&self) -> Option<&ObjectId> {
        self.parent.as_ref()
    }

    pub fn author(&self) -> &Author {
        &self.author
    }
}

impl Packable for Commit {
    fn serialize(&self) -> anyhow::Result<Bytes> {
        let mut object_content = vec![];

        object_content.push(format!("tree {}", self.tree_oid.as_ref()));
        if let Some(parent) = &self.parent {
            object_content.push(format!("parent {}", parent.as_ref()));
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

impl Unpackable for Commit {
    fn deserialize(reader: impl BufRead) -> anyhow::Result<Self> {
        let content = reader
            .bytes()
            .collect::<Result<Vec<u8>, std::io::Error>>()?;

        let content = String::from_utf8(content)?;
        let mut lines = content.lines();

        let tree_line = lines
            .next()
            .context("Invalid commit object: missing tree line")?;
        let tree_oid = tree_line
            .strip_prefix("tree ")
            .context("Invalid commit object: invalid tree line")?
            .to_string();
        let tree_oid = ObjectId::try_parse(tree_oid)?;

        let parent_line = lines
            .next()
            .context("Invalid commit object: missing parent line")?;
        let parent = if parent_line.starts_with("parent ") {
            Some(ObjectId::try_parse(
                parent_line.strip_prefix("parent ").unwrap().to_string(),
            )?)
        } else {
            None
        };

        let author_line = if parent.is_some() {
            lines
                .next()
                .context("Invalid commit object: missing author line")?
        } else {
            parent_line
        };
        let author = author_line
            .strip_prefix("author ")
            .context("Invalid commit object: invalid author line")?;
        let author = Author::try_from(author)?;

        let committer_line = lines
            .next()
            .context("Invalid commit object: missing committer line")?;
        let committer = committer_line
            .strip_prefix("committer ")
            .context("Invalid commit object: invalid committer line")?;
        let _committer = Author::try_from(committer)?;

        // skip the empty line
        lines.next();

        let message = lines.collect::<Vec<&str>>().join("\n");
        Ok(Self::new(parent, tree_oid, author, message))
    }
}

impl Object for Commit {
    fn object_type(&self) -> ObjectType {
        ObjectType::Commit
    }

    fn display(&self) -> String {
        let mut lines = vec![];

        lines.push(format!("tree {}", self.tree_oid.as_ref()));
        if let Some(parent) = &self.parent {
            lines.push(format!("parent {}", parent.as_ref()));
        }
        lines.push(format!("author {}", self.author.display()));
        lines.push(format!("committer {}", self.committer.display()));
        lines.push(String::new());
        lines.push(self.message.to_string());

        lines.join("\n")
    }
}
