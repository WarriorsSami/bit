use crate::artifacts::diff::tree_diff::TreeDiff;
use crate::artifacts::log::path_filter::PathFilter;
use crate::artifacts::objects::blob::Blob;
use crate::artifacts::objects::commit::Commit;
use crate::artifacts::objects::object::{Object, ObjectBox, Unpackable};
use crate::artifacts::objects::object_id::ObjectId;
use crate::artifacts::objects::object_type::ObjectType;
use crate::artifacts::objects::tree::Tree;
use anyhow::Context;
use bytes::Bytes;
use fake::rand;
use std::io::{BufRead, Cursor, Read, Write};
use std::path::{Path, PathBuf};

#[derive(Debug)]
pub struct Database {
    path: Box<Path>,
}

// TODO: implement packfiles for better performance and storage efficiency
// TODO: refactor to use async fs operations
impl Database {
    pub fn new(path: Box<Path>) -> Self {
        Database { path }
    }

    pub fn objects_path(&self) -> &Path {
        &self.path
    }

    pub fn tree_diff(
        &self,
        old_oid: Option<&ObjectId>,
        new_oid: Option<&ObjectId>,
        path_filter: PathFilter,
    ) -> anyhow::Result<TreeDiff<'_>> {
        let mut tree_diff = TreeDiff::new(self);
        tree_diff.compare_oids(old_oid, new_oid, path_filter)?;
        Ok(tree_diff)
    }

    pub fn load(&self, object_id: &ObjectId) -> anyhow::Result<Bytes> {
        let object_path = self.path.join(object_id.to_path());

        self.read_object(object_path)
    }

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

    pub fn parse_object_as_blob(&self, object_id: &ObjectId) -> anyhow::Result<Option<Blob>> {
        let (object_type, object_reader) = self.parse_object_as_bytes(object_id)?;

        match object_type {
            ObjectType::Blob => Ok(Some(Blob::deserialize(object_reader)?)),
            _ => Ok(None),
        }
    }

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
