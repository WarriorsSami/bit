use crate::domain::objects::entry::FileMode;
use crate::domain::objects::object::Object;
use crate::domain::objects::object_type::ObjectType;
use bytes::Bytes;

#[derive(Debug, Clone)]
pub struct Blob<'blob> {
    content: &'blob str,
    stat: FileMode,
}

impl<'blob> Blob<'blob> {
    pub fn new(content: &'blob str, stat: FileMode) -> Self {
        Blob { content, stat }
    }

    pub fn mode(&self) -> &FileMode {
        &self.stat
    }

    fn from(data: &'blob str) -> anyhow::Result<Self> {
        let parts = data.splitn(2, '\0').collect::<Vec<&str>>();

        if parts.len() != 2 {
            return Err(anyhow::anyhow!("Invalid blob file"));
        }

        Ok(Self::new(parts[1], Default::default()))
    }
}

impl<'blob> TryFrom<&'blob str> for Blob<'blob> {
    type Error = anyhow::Error;

    fn try_from(data: &'blob str) -> anyhow::Result<Self> {
        Blob::from(data)
    }
}

impl<'blob> Object for Blob<'_> {
    fn serialize(&self) -> anyhow::Result<Bytes> {
        let object_content = format!(
            "{} {}\0{}",
            self.object_type().as_str(),
            self.content.len(),
            self.content
        );

        Ok(Bytes::from(object_content))
    }

    fn object_type(&self) -> ObjectType {
        ObjectType::Blob
    }

    fn display(&self) -> String {
        self.content.to_string()
    }
}
