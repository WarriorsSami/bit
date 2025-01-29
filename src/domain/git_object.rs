use crate::domain::object_type::ObjectType;
use crate::domain::ByteArray;
use anyhow::{Context, Result};
use sha1::{Digest, Sha1};
use std::io::{Read, Write};

pub trait GitObject {
    fn serialize(&self) -> Result<ByteArray>;
    fn deserialize(data: ByteArray) -> Result<Self>
    where
        Self: Sized;

    fn compress(&self) -> Result<ByteArray> {
        let object_content = self.serialize()?;
        let mut encoder =
            flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::default());
        encoder
            .write_all(&object_content)
            .context("Unable to compress object content")?;

        encoder
            .finish()
            .map(|compressed_content| compressed_content.into())
            .context("Unable to finish compressing object content")
    }
    fn decompress(data: ByteArray) -> Result<ByteArray> {
        let mut decoder = flate2::read::ZlibDecoder::new(&*data);
        let mut decompressed_content = Vec::new();
        decoder
            .read_to_end(&mut decompressed_content)
            .context("Unable to decompress object content")?;

        Ok(decompressed_content.into())
    }

    fn object_type(&self) -> ObjectType;
    fn object_id(&self) -> Result<String> {
        let content = self.serialize()?;
        let mut hasher = Sha1::new();
        hasher.update(&content);

        Ok(format!("{:x}", hasher.finalize()))
    }
}
