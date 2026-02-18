//! Git commit object
//!
//! Commits represent snapshots of the repository at specific points in time.
//! They contain:
//! - A tree object ID (directory snapshot)
//! - Parent commit ID(s) (for history)
//! - Author and committer information
//! - Commit message
//!
//! ## Format
//!
//! On disk:
//! ```text
//! commit <size>\0
//! tree <tree-sha>
//! parent <parent-sha>
//! author <name> <email> <timestamp> <timezone>
//! committer <name> <email> <timestamp> <timezone>
//!
//! <commit message>
//! ```

use crate::artifacts::objects::object::Unpackable;
use crate::artifacts::objects::object::{Object, Packable};
use crate::artifacts::objects::object_id::ObjectId;
use crate::artifacts::objects::object_type::ObjectType;
use anyhow::Context;
use bytes::Bytes;
use std::io::{BufRead, Write};

/// Author or committer information
///
/// Contains name, email, and timestamp with timezone information.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Author {
    name: String,
    email: String,
    timestamp: chrono::DateTime<chrono::FixedOffset>,
}

impl Author {
    /// Create a new author with the current timestamp
    ///
    /// # Arguments
    ///
    /// * `name` - Author's name
    /// * `email` - Author's email address
    pub fn new(name: String, email: String) -> Self {
        Author {
            name,
            email,
            timestamp: chrono::Local::now().fixed_offset(),
        }
    }

    /// Create a new author with a specific timestamp
    ///
    /// # Arguments
    ///
    /// * `name` - Author's name
    /// * `email` - Author's email address  
    /// * `timestamp` - Specific timestamp with timezone
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

    /// Format author name and email for display
    ///
    /// # Returns
    ///
    /// String in format "Name <email@example.com>"
    pub fn display_name(&self) -> String {
        format!("{} <{}>", self.name, self.email)
    }

    /// Format complete author info including timestamp
    ///
    /// # Returns
    ///
    /// String in format "Name <email> timestamp timezone"
    pub fn display(&self) -> String {
        format!(
            "{} <{}> {} {}",
            self.name,
            self.email,
            self.timestamp.timestamp(),
            self.timestamp.format("%z")
        )
    }

    /// Load author information from environment variables
    ///
    /// Reads GIT_AUTHOR_NAME, GIT_AUTHOR_EMAIL, and optionally GIT_AUTHOR_DATE.
    /// If no date is provided, uses current time.
    ///
    /// # Returns
    ///
    /// Author struct populated from environment
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

    /// Format timestamp in human-readable form
    ///
    /// # Returns
    ///
    /// String like "Mon Jan 1 12:34:56 2024 +0000"
    pub fn readable_timestamp(&self) -> String {
        self.timestamp
            .format("%a %b %-d %H:%M:%S %Y %z")
            .to_string()
    }

    /// Get the timestamp
    pub fn timestamp(&self) -> chrono::DateTime<chrono::FixedOffset> {
        self.timestamp
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

/// Slim representation of a commit
///
/// Contains only essential information for lightweight operations like merge base finding.
/// This struct owns its data to allow for lazy loading from a cache with interior mutability.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct SlimCommit {
    /// The commit's object ID
    pub oid: ObjectId,
    /// The commit's parent object IDs
    pub parents: Vec<ObjectId>,
    /// Commit timestamp (needed for comparison)
    pub timestamp: chrono::DateTime<chrono::FixedOffset>,
}

impl PartialOrd for SlimCommit {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for SlimCommit {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.timestamp.cmp(&other.timestamp)
    }
}

// Note: SlimCommit is now created by borrowing from a commit cache in the Database.
// The Database provides a method to create SlimCommit instances that borrow from its cache.

/// Git commit object
///
/// Represents a snapshot of the repository with metadata.
/// Contains references to:
/// - The tree representing the state of files
/// - Parent commit(s) for history
/// - Author and committer information
/// - Commit message
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Commit {
    /// Parent commit IDs (empty for initial commit, multiple for merge commits)
    parents: Vec<ObjectId>,
    /// Tree object ID representing the directory snapshot
    tree_oid: ObjectId,
    /// Author who wrote the changes
    author: Author,
    /// Committer who recorded the commit
    committer: Author,
    /// Commit message
    message: String,
}

impl Commit {
    /// Create a new commit
    ///
    /// # Arguments
    ///
    /// * `parent` - Parent commit ID (None for initial commit)
    /// * `tree_oid` - Tree object representing the snapshot
    /// * `author` - Author (also used as committer)
    /// * `message` - Commit message
    pub fn new(
        parents: Vec<ObjectId>,
        tree_oid: ObjectId,
        author: Author,
        message: String,
    ) -> Self {
        Commit {
            parents,
            tree_oid,
            author: author.clone(),
            committer: author,
            message,
        }
    }

    /// Get the first line of the commit message
    ///
    /// Useful for short-form display (e.g., `git log --oneline`)
    pub fn short_message(&self) -> String {
        self.message.lines().next().unwrap_or("").to_string()
    }

    /// Get the full commit message
    pub fn message(&self) -> &str {
        &self.message
    }

    /// Get the tree object ID
    pub fn tree_oid(&self) -> &ObjectId {
        &self.tree_oid
    }

    pub fn parent(&self) -> Option<&ObjectId> {
        self.parents.first()
    }

    pub fn author(&self) -> &Author {
        &self.author
    }

    pub fn timestamp(&self) -> chrono::DateTime<chrono::FixedOffset> {
        self.author.timestamp()
    }
}

impl Packable for Commit {
    fn serialize(&self) -> anyhow::Result<Bytes> {
        let mut object_content = vec![];

        object_content.push(format!("tree {}", self.tree_oid.as_ref()));
        for parent in &self.parents {
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

        // Parse all parent lines (there can be 0, 1, or multiple parents)
        let mut parents = Vec::new();
        let mut next_line = lines
            .next()
            .context("Invalid commit object: missing author line")?;

        while next_line.starts_with("parent ") {
            let parent_oid = next_line
                .strip_prefix("parent ")
                .context("Invalid commit object: invalid parent line")?;
            parents.push(ObjectId::try_parse(parent_oid.to_string())?);

            next_line = lines
                .next()
                .context("Invalid commit object: missing author line")?;
        }

        // At this point, next_line should be the author line
        let author = next_line
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
        Ok(Self::new(parents, tree_oid, author, message))
    }
}

impl Object for Commit {
    fn object_type(&self) -> ObjectType {
        ObjectType::Commit
    }

    fn display(&self) -> String {
        let mut lines = vec![];

        lines.push(format!("tree {}", self.tree_oid.as_ref()));
        for parent in &self.parents {
            lines.push(format!("parent {}", parent.as_ref()));
        }
        lines.push(format!("author {}", self.author.display()));
        lines.push(format!("committer {}", self.committer.display()));
        lines.push(String::new());
        lines.push(self.message.to_string());

        lines.join("\n")
    }
}
