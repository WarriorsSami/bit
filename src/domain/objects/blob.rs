use crate::domain::objects::object::Object;
use crate::domain::objects::object_type::ObjectType;
use crate::domain::ByteArray;

pub struct Blob {
    content: ByteArray,
}

impl Blob {
    pub fn new(content: ByteArray) -> Self {
        Blob { content }
    }

    pub fn pretty_print(&self) -> String {
        String::from_utf8_lossy(&self.content).to_string()
    }
}

impl Object for Blob {
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

    fn object_type(&self) -> ObjectType {
        ObjectType::Blob
    }
}
