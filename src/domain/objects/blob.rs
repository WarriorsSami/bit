use crate::domain::objects::entry_mode::FileMode;
use crate::domain::objects::object::{Object, Packable};
use crate::domain::objects::object_type::ObjectType;
use bytes::Bytes;
use derive_new::new;
use std::io::Write;

#[derive(Debug, Clone, new)]
pub struct Blob<'blob> {
    content: &'blob str,
    stat: FileMode,
}

impl Blob<'_> {
    pub fn mode(&self) -> &FileMode {
        &self.stat
    }
}

// TODO: Convert from Bytes instead of &str
impl<'blob> TryFrom<&'blob str> for Blob<'blob> {
    type Error = anyhow::Error;

    fn try_from(data: &'blob str) -> anyhow::Result<Self> {
        let parts = data.splitn(2, '\0').collect::<Vec<&str>>();

        if parts.len() != 2 {
            return Err(anyhow::anyhow!("Invalid blob file"));
        }

        Ok(Self::new(parts[1], Default::default()))
    }
}

impl Packable for Blob<'_> {
    fn serialize(&self) -> anyhow::Result<Bytes> {
        let mut content_bytes = Vec::new();
        content_bytes.write_all(self.content.as_bytes())?;

        let mut blob_bytes = Vec::new();
        let header = format!("{} {}\0", self.object_type().as_str(), content_bytes.len());
        blob_bytes.write_all(header.as_bytes())?;
        blob_bytes.write_all(&content_bytes)?;

        Ok(Bytes::from(blob_bytes))
    }
}

impl Object for Blob<'_> {
    fn object_type(&self) -> ObjectType {
        ObjectType::Blob
    }

    fn display(&self) -> String {
        self.content.to_string()
    }
}
