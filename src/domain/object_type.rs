pub enum ObjectType {
    Blob,
}

impl ObjectType {
    pub fn as_str(&self) -> &str {
        match self {
            ObjectType::Blob => "blob",
        }
    }
}
