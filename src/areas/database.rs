//! Object database for Git objects
//!
//! The database stores all Git objects (blobs, trees, commits) using content-addressable storage.
//! Objects are identified by their SHA-1 hash and stored in a directory structure based on
//! the hash prefix for efficient lookup.
//!
//! ## Storage Format
//!
//! Objects are stored as:
//! - Path: `.git/objects/ab/cdef123...` (first 2 chars as directory, rest as filename)
//! - Content: Compressed (zlib) format containing type, size, and data

use crate::artifacts::diff::tree_diff::TreeDiff;
use crate::artifacts::log::path_filter::PathFilter;
use crate::artifacts::objects::blob::Blob;
use crate::artifacts::objects::commit::{Commit, SlimCommit};
use crate::artifacts::objects::object::{Object, ObjectBox, Unpackable};
use crate::artifacts::objects::object_id::ObjectId;
use crate::artifacts::objects::object_type::ObjectType;
use crate::artifacts::objects::tree::Tree;
use anyhow::Context;
use bytes::Bytes;
use fake::rand;
use std::collections::HashMap;
use std::io::{BufRead, Cursor, Read, Write};
use std::path::{Path, PathBuf};

/// Cached commit data for efficient borrowing
///
/// This struct stores the essential commit information in a format
/// optimized for creating borrowed SlimCommit instances.
#[derive(Debug, Clone)]
struct CachedCommit {
    /// The commit's object ID
    oid: ObjectId,
    /// Parent commit object IDs
    parents: Vec<ObjectId>,
    /// Commit timestamp
    timestamp: chrono::DateTime<chrono::FixedOffset>,
}

/// Git object database
///
/// Manages storage and retrieval of content-addressable objects.
/// All objects are identified by their SHA-1 hash and stored in compressed format.
#[derive(Debug)]
pub struct Database {
    /// Path to the objects directory (typically `.git/objects`)
    path: Box<Path>,
}

// TODO: implement packfiles for better performance and storage efficiency
// TODO: refactor to use async fs operations
impl Database {
    /// Create a new database instance
    ///
    /// # Arguments
    ///
    /// * `path` - Path to the objects directory (typically `.git/objects`)
    pub fn new(path: Box<Path>) -> Self {
        Database { path }
    }

    /// Get the path to the objects directory
    pub fn objects_path(&self) -> &Path {
        &self.path
    }

    /// Create a tree diff between two commits
    ///
    /// # Arguments
    ///
    /// * `old_oid` - Object ID of the old tree (None for empty tree)
    /// * `new_oid` - Object ID of the new tree (None for empty tree)
    /// * `path_filter` - Filter to limit diff to specific paths
    ///
    /// # Returns
    ///
    /// A TreeDiff containing all changes between the two trees
    pub fn tree_diff(
        &self,
        old_oid: Option<&ObjectId>,
        new_oid: Option<&ObjectId>,
        path_filter: &PathFilter,
    ) -> anyhow::Result<TreeDiff<'_>> {
        let mut tree_diff = TreeDiff::new(self);
        tree_diff.compare_oids(old_oid, new_oid, path_filter)?;
        Ok(tree_diff)
    }

    /// Load raw object bytes from the database
    ///
    /// # Arguments
    ///
    /// * `object_id` - The SHA-1 hash identifying the object
    ///
    /// # Returns
    ///
    /// The decompressed object content including header
    pub fn load(&self, object_id: &ObjectId) -> anyhow::Result<Bytes> {
        let object_path = self.path.join(object_id.to_path());

        self.read_object(object_path)
    }

    /// Store an object in the database
    ///
    /// The object is serialized, and its content is written to the appropriate
    /// path based on its SHA-1 hash. If the object already exists, this is a no-op.
    ///
    /// # Arguments
    ///
    /// * `object` - Any object implementing the Object trait
    ///
    /// # Returns
    ///
    /// Ok(()) if successful, error if storage fails
    pub fn store(&self, object: impl Object) -> anyhow::Result<()> {
        let object_path = self.path.join(object.object_path()?);
        let object_content = object.serialize()?;

        // write the object to disk unless it already exists
        // otherwise, create the object directory
        if !object_path.exists() {
            std::fs::create_dir_all(
                object_path
                    .parent()
                    .context(format!("Invalid object path {}", object_path.display()))?,
            )
            .context(format!(
                "Unable to create object directory {}",
                object_path.display()
            ))?;

            self.write_object(object_path, object_content)?;
        }

        Ok(())
    }

    /// Parse an object from the database into the appropriate type
    ///
    /// Loads the object, determines its type, and deserializes it into
    /// the corresponding struct (Blob, Tree, or Commit).
    ///
    /// # Arguments
    ///
    /// * `object_id` - The SHA-1 hash identifying the object
    ///
    /// # Returns
    ///
    /// An ObjectBox enum containing the parsed object
    pub fn parse_object(&self, object_id: &ObjectId) -> anyhow::Result<ObjectBox<'_>> {
        let (object_type, object_reader) = self.parse_object_as_bytes(object_id)?;

        match object_type {
            ObjectType::Blob => {
                // parse as blob
                Ok(ObjectBox::Blob(Box::new(Blob::deserialize(object_reader)?)))
            }
            ObjectType::Tree => {
                // parse as tree
                Ok(ObjectBox::Tree(Box::new(Tree::deserialize(object_reader)?)))
            }
            ObjectType::Commit => {
                // parse as commit
                Ok(ObjectBox::Commit(Box::new(Commit::deserialize(
                    object_reader,
                )?)))
            }
        }
    }

    /// Parse an object as a Blob, if it is one
    ///
    /// # Returns
    ///
    /// Some(Blob) if the object is a blob, None otherwise
    pub fn parse_object_as_blob(&self, object_id: &ObjectId) -> anyhow::Result<Option<Blob>> {
        let (object_type, object_reader) = self.parse_object_as_bytes(object_id)?;

        match object_type {
            ObjectType::Blob => Ok(Some(Blob::deserialize(object_reader)?)),
            _ => Ok(None),
        }
    }

    /// Parse an object as a Tree, if it is one
    ///
    /// # Returns
    ///
    /// Some(Tree) if the object is a tree, None otherwise
    pub fn parse_object_as_tree(&self, object_id: &ObjectId) -> anyhow::Result<Option<Tree<'_>>> {
        let (object_type, object_reader) = self.parse_object_as_bytes(object_id)?;

        match object_type {
            ObjectType::Tree => {
                // parse as tree
                Ok(Some(Tree::deserialize(object_reader)?))
            }
            _ => Ok(None),
        }
    }

    /// Parse an object as a Commit, if it is one
    ///
    /// # Returns
    ///
    /// Some(Commit) if the object is a commit, None otherwise
    pub fn parse_object_as_commit(&self, object_id: &ObjectId) -> anyhow::Result<Option<Commit>> {
        let (object_type, object_reader) = self.parse_object_as_bytes(object_id)?;

        match object_type {
            ObjectType::Commit => {
                // parse as commit
                Ok(Some(Commit::deserialize(object_reader)?))
            }
            _ => Ok(None),
        }
    }

    fn parse_object_as_bytes(
        &self,
        object_id: &ObjectId,
    ) -> anyhow::Result<(ObjectType, impl BufRead)> {
        let object_path = self.path.join(object_id.to_path());
        let object_content = self.read_object(object_path)?;
        let mut object_reader = Cursor::new(object_content);

        let object_type = ObjectType::parse_object_type(&mut object_reader)?;

        Ok((object_type, object_reader))
    }

    fn read_object(&self, object_path: PathBuf) -> anyhow::Result<Bytes> {
        // read the object file
        let object_content = std::fs::read(&object_path).context(format!(
            "Unable to read object file {}",
            object_path.display()
        ))?;

        // decompress the object content
        let object_content = Self::decompress(object_content.into())?;

        // return the object content
        Ok(object_content)
    }

    fn write_object(&self, object_path: PathBuf, object_content: Bytes) -> anyhow::Result<()> {
        let object_dir = object_path
            .parent()
            .context(format!("Invalid object path {}", object_path.display()))?;
        let temp_object_path = object_dir.join(Self::generate_temp_name());

        // compress the object content
        let object_content = Self::compress(object_content)?;

        // open the file as RDWR, CREAT and EXCL
        // if ENOENT, create the parent directory and open the file with the same flags
        let mut file = std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(true)
            .open(&temp_object_path)
            .context(format!(
                "Unable to open object file {}",
                temp_object_path.display()
            ))?;

        file.write_all(&object_content).context(format!(
            "Unable to write object file {}",
            temp_object_path.display()
        ))?;

        // rename the temp file to the object file to make it atomic
        std::fs::rename(&temp_object_path, &object_path).context(format!(
            "Unable to rename object file to {}",
            object_path.display()
        ))?;

        Ok(())
    }

    fn compress(data: Bytes) -> anyhow::Result<Bytes> {
        let mut encoder =
            flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::default());
        encoder
            .write_all(&data)
            .context("Unable to compress object content")?;

        encoder
            .finish()
            .map(|compressed_content| compressed_content.into())
            .context("Unable to finish compressing object content")
    }

    fn decompress(data: Bytes) -> anyhow::Result<Bytes> {
        let mut decoder = flate2::read::ZlibDecoder::new(&*data);
        let mut decompressed_content = Vec::new();
        decoder
            .read_to_end(&mut decompressed_content)
            .context("Unable to decompress object content")?;

        Ok(decompressed_content.into())
    }

    fn generate_temp_name() -> String {
        format!("tmp-obj-{}", rand::random::<u32>())
    }

    /// Find all objects whose OID starts with the given prefix.
    ///
    /// This method searches the object database for all objects whose OID begins
    /// with the specified prefix. It's used to resolve abbreviated OIDs to their
    /// full form.
    ///
    /// # Arguments
    ///
    /// * `prefix` - A hexadecimal string prefix (e.g., "abc", "a1b2c3")
    ///
    /// # Returns
    ///
    /// A vector of all matching ObjectIds. If no matches are found, returns an empty vector.
    /// If multiple matches are found, all are returned (indicating an ambiguous prefix).
    ///
    /// # Performance
    ///
    /// - For prefixes of 2+ characters, only searches the specific directory
    /// - For prefixes of 0-1 characters, must search all directories (slower)
    pub fn find_objects_by_prefix(&self, prefix: &str) -> anyhow::Result<Vec<ObjectId>> {
        let mut matches = Vec::new();

        // Determine which directory to search
        // If prefix is less than 2 chars, we'd need to search all dirs (0-ff)
        // If prefix is 2+ chars, we only search the specific directory
        if prefix.len() >= 2 {
            let dir_name = &prefix[..2];
            let file_prefix = &prefix[2..];
            let dir_path = self.path.join(dir_name);

            if dir_path.exists() && dir_path.is_dir() {
                for entry in std::fs::read_dir(&dir_path)? {
                    let entry = entry?;
                    let file_name = entry.file_name();
                    let file_name_str = file_name.to_string_lossy();

                    if file_name_str.starts_with(file_prefix) {
                        let full_oid = format!("{}{}", dir_name, file_name_str);
                        if let Ok(oid) = ObjectId::try_parse(full_oid) {
                            matches.push(oid);
                        }
                    }
                }
            }
        } else {
            // Search all directories
            for i in 0..=255 {
                let dir_name = format!("{:02x}", i);
                let dir_path = self.path.join(&dir_name);

                if dir_path.exists() && dir_path.is_dir() {
                    for entry in std::fs::read_dir(&dir_path)? {
                        let entry = entry?;
                        let file_name = entry.file_name();
                        let file_name_str = file_name.to_string_lossy();
                        let full_oid = format!("{}{}", dir_name, file_name_str);

                        if full_oid.starts_with(prefix) {
                            let oid = ObjectId::try_parse(full_oid)?;
                            matches.push(oid);
                        }
                    }
                }
            }
        }

        Ok(matches)
    }

    /// Get the type of an object as a string.
    ///
    /// Returns the object type ("blob", "tree", or "commit") for the given object ID.
    /// This is useful for displaying object information to the user, especially when
    /// showing ambiguous OID candidates.
    ///
    /// # Arguments
    ///
    /// * `object_id` - The full object ID to check
    ///
    /// # Returns
    ///
    /// A string representing the object type: "blob", "tree", or "commit"
    pub fn get_object_type(&self, object_id: &ObjectId) -> anyhow::Result<ObjectType> {
        let (object_type, _) = self.parse_object_as_bytes(object_id)?;
        Ok(object_type)
    }
}

/// Commit cache for efficient borrowing during graph traversal algorithms
///
/// This cache stores loaded commits in a way that allows creating SlimCommit instances
/// that borrow from the cache instead of owning their data. This is particularly useful
/// for algorithms like merge base finding that may access the same commits multiple times.
///
/// # Lifetime Management
///
/// The cache stores commits with their ObjectIds and parent lists. SlimCommit instances
/// created from this cache borrow references to these stored values, so they must not
/// outlive the cache itself.
///
/// # Usage Pattern
///
/// ```rust,ignore
/// let mut cache = CommitCache::new();
///
/// // Populate the cache as needed during traversal
/// cache.load_commit(&database, &commit_id)?;
///
/// // Get borrowed SlimCommit instances
/// let slim = cache.get_slim_commit(&commit_id)?;
///
/// // Use with BCA finder
/// let finder = BCAFinder::new(|oid| cache.get_slim_commit(oid).unwrap());
/// ```
#[derive(Debug)]
pub struct CommitCache {
    /// Map from commit OID to cached commit data
    commits: HashMap<ObjectId, CachedCommit>,
}

impl CommitCache {
    /// Create a new empty commit cache
    pub fn new() -> Self {
        Self {
            commits: HashMap::new(),
        }
    }

    /// Load a commit into the cache if not already present
    ///
    /// # Arguments
    ///
    /// * `database` - The database to load the commit from
    /// * `object_id` - The commit's object ID
    ///
    /// # Returns
    ///
    /// Ok(()) if successful, or an error if the object doesn't exist or isn't a commit
    pub fn load_commit(&mut self, database: &Database, object_id: &ObjectId) -> anyhow::Result<()> {
        if self.commits.contains_key(object_id) {
            return Ok(()); // Already cached
        }

        let commit = database
            .parse_object_as_commit(object_id)?
            .ok_or_else(|| anyhow::anyhow!("Object {} is not a commit", object_id))?;

        let cached = CachedCommit {
            oid: commit.object_id()?,
            parents: commit.parent().cloned().into_iter().collect(),
            timestamp: commit.timestamp(),
        };

        self.commits.insert(object_id.clone(), cached);
        Ok(())
    }

    /// Get a SlimCommit that borrows from this cache
    ///
    /// The commit must already be loaded into the cache via `load_commit`.
    ///
    /// # Lifetime
    ///
    /// The returned SlimCommit borrows from the cache, so it must not outlive
    /// the cache instance. The lifetime `'cache` ties the SlimCommit to this cache.
    ///
    /// # Arguments
    ///
    /// * `object_id` - The commit's object ID
    ///
    /// # Returns
    ///
    /// A SlimCommit borrowing data from the cache, or an error if the commit
    /// is not in the cache
    pub fn get_slim_commit(&'_ self, object_id: &ObjectId) -> anyhow::Result<SlimCommit<'_>> {
        let cached = self
            .commits
            .get(object_id)
            .ok_or_else(|| anyhow::anyhow!("Commit {} not found in cache", object_id))?;

        Ok(SlimCommit {
            oid: &cached.oid,
            parents: &cached.parents,
            timestamp: cached.timestamp,
        })
    }

    /// Get a SlimCommit, loading it from the database if necessary
    ///
    /// This is a convenience method that combines `load_commit` and `get_slim_commit`.
    ///
    /// # Arguments
    ///
    /// * `database` - The database to load from if needed
    /// * `object_id` - The commit's object ID
    ///
    /// # Returns
    ///
    /// A SlimCommit borrowing data from the cache
    pub fn get_or_load_slim_commit<'cache>(
        &'cache mut self,
        database: &Database,
        object_id: &ObjectId,
    ) -> anyhow::Result<SlimCommit<'cache>> {
        self.load_commit(database, object_id)?;
        self.get_slim_commit(object_id)
    }
}

impl Default for CommitCache {
    fn default() -> Self {
        Self::new()
    }
}
