use crate::domain::git_object::GitObject;
use crate::domain::object_type::ObjectType;
use crate::domain::ByteArray;

pub struct BlobObject {
    content: ByteArray,
}

impl BlobObject {
    pub fn new(content: ByteArray) -> Self {
        BlobObject { content }
    }

    pub fn pretty_print(&self) -> String {
        String::from_utf8_lossy(&self.content).to_string()
    }
}

impl GitObject for BlobObject {
    fn serialize(&self) -> anyhow::Result<ByteArray> {
        let object_content = format!(
            "{} {}\0{}",
            ObjectType::Blob.as_str(),
            self.content.len(),
            String::from_utf8_lossy(&self.content)
        );
        let object_raw_content = object_content.as_bytes();

        Ok(object_raw_content.into())
    }

    fn deserialize(data: ByteArray) -> anyhow::Result<Self> {
        let parts: Vec<&[u8]> = data.splitn(2, |&b| b == 0).collect();

        if parts.len() != 2 {
            return Err(anyhow::anyhow!("Invalid blob object"));
        }

        Ok(BlobObject {
            content: parts[1].into(),
        })
    }

    fn object_type(&self) -> ObjectType {
        ObjectType::Blob
    }
}
