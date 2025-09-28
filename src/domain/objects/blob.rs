use crate::domain::objects::core::entry_mode::FileMode;
use crate::domain::objects::core::object::{Object, Packable};
use crate::domain::objects::core::object_type::ObjectType;
use crate::domain::objects::object::Unpackable;
use bytes::Bytes;
use derive_new::new;
use std::io::{BufRead, Write};

#[derive(Debug, Clone, new)]
pub struct Blob {
    content: String,
    stat: FileMode,
}

impl Blob {
    pub fn mode(&self) -> &FileMode {
        &self.stat
    }
}

impl Packable for Blob {
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

impl Unpackable for Blob {
    fn deserialize(reader: impl BufRead) -> anyhow::Result<Self> {
        // the header has already been read
        let content = reader
            .bytes()
            .collect::<Result<Vec<u8>, std::io::Error>>()?;

        let content = String::from_utf8(content)?;
        Ok(Self::new(content, Default::default()))
    }
}

impl Object for Blob {
    fn object_type(&self) -> ObjectType {
        ObjectType::Blob
    }

    fn display(&self) -> String {
        self.content.to_string()
    }
}
