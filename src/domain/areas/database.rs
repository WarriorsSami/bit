use crate::domain::objects::object::Object;
use anyhow::Context;
use bytes::Bytes;
use fake::rand;
use std::io::{Read, Write};
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

    pub fn load(&self, object_id: &str) -> anyhow::Result<Bytes> {
        let object_path = self
            .path
            .join(Path::new(&object_id[..2]).join(Path::new(&object_id[2..])));

        self.read_object(object_path)
    }

    pub fn store(&self, object: impl Object) -> anyhow::Result<()> {
        let object_path = self.path.join(object.object_path()?);
        let object_content = object.serialize()?;

        self.write_object(object_path, object_content)?;

        Ok(())
    }

    fn read_object(&self, object_path: PathBuf) -> anyhow::Result<Bytes> {
        // read the object file
        let object_content = std::fs::read(&object_path).context(format!(
            "Unable to read object file {}",
            object_path.display()
        ))?;

        // decompress the object content
        let object_content = Self::decompress(object_content.into())?;

        // extract the object content by removing the header
        let parts = object_content
            .splitn(2, |&byte| byte == 0)
            .map(|part| part.to_vec())
            .collect::<Vec<_>>();

        if parts.len() != 2 {
            return Err(anyhow::anyhow!("Invalid object file"));
        }

        Ok(Bytes::from(parts[1].clone()))
    }

    fn write_object(&self, object_path: PathBuf, object_content: Bytes) -> anyhow::Result<()> {
        let object_dir = object_path.parent().context(format!("Invalid object path {}", object_path.display()))?;
        let temp_object_path = object_dir.join(Self::generate_temp_name());

        // create the object directory if it doesn't exist
        if !object_dir.exists() {
            std::fs::create_dir_all(object_dir).context(format!(
                "Unable to create object directory {}",
                object_dir.display()
            ))?;
        }

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

        file.write_all(&object_content)
            .context(format!("Unable to write object file {}", temp_object_path.display()))?;

        // rename the temp file to the object file to make it atomic
        std::fs::rename(&temp_object_path, &object_path)
            .context(format!("Unable to rename object file to {}", object_path.display()))?;

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
