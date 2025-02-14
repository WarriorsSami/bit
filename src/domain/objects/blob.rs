use crate::domain::objects::object::Object;
use crate::domain::objects::object_type::ObjectType;
use bytes::Bytes;

#[derive(Debug, Clone)]
pub struct Blob {
    content: String,
}

impl Blob {
    pub fn new(content: String) -> Self {
        Blob { content }
    }

    pub fn display(&self) -> String {
        self.content.clone()
    }
}

impl Object for Blob {
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
}
