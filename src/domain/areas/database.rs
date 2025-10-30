use crate::domain::objects::blob::Blob;
use crate::domain::objects::commit::Commit;
use crate::domain::objects::object::{Object, Unpackable};
use crate::domain::objects::object_id::ObjectId;
use crate::domain::objects::object_type::ObjectType;
use crate::domain::objects::tree::Tree;
use anyhow::Context;
use bytes::Bytes;
use fake::rand;
use std::io::{BufRead, Cursor, Read, Write};
use std::path::{Path, PathBuf};

pub struct Database {
    path: Box<Path>,
}

impl Database {
    pub fn new(path: Box<Path>) -> Self {
        Database { path }
    }

    pub fn objects_path(&self) -> &Path {
        &self.path
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

    pub fn parse_object(&self, object_id: &ObjectId) -> anyhow::Result<Box<dyn Object>> {
        let (object_type, object_reader) = self.parse_object_as_bytes(object_id)?;

        match object_type {
            ObjectType::Blob => {
                // parse as blob
                Ok(Box::new(Blob::deserialize(object_reader)?))
            }
            ObjectType::Tree => {
                // parse as tree
                Ok(Box::new(Tree::deserialize(object_reader)?))
            }
            ObjectType::Commit => {
                // parse as commit
                Ok(Box::new(Commit::deserialize(object_reader)?))
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
}
