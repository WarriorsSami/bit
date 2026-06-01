use std::io::BufRead;

#[derive(Debug, thiserror::Error)]
pub enum ObjectTypeError {
    #[error("invalid object type: {0}")]
    Invalid(String),
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error("invalid UTF-8 in object type header")]
    InvalidUtf8(#[from] std::string::FromUtf8Error),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ObjectType {
    Blob,
    Tree,
    Commit,
}

impl ObjectType {
    pub fn as_str(&self) -> &str {
        match self {
            ObjectType::Blob => "blob",
            ObjectType::Tree => "tree",
            ObjectType::Commit => "commit",
        }
    }

    pub fn parse_object_type(
        data_reader: &mut impl BufRead,
    ) -> Result<ObjectType, ObjectTypeError> {
        let mut object_type = Vec::new();
        data_reader.read_until(b' ', &mut object_type)?;

        let object_type = String::from_utf8(object_type)?;
        let object_type = object_type.trim();

        let mut size = Vec::new();
        data_reader.read_until(b'\0', &mut size)?;

        ObjectType::try_from(object_type)
    }
}

impl TryFrom<&str> for ObjectType {
    type Error = ObjectTypeError;

    fn try_from(value: &str) -> Result<Self, ObjectTypeError> {
        match value {
            "blob" => Ok(ObjectType::Blob),
            "tree" => Ok(ObjectType::Tree),
            "commit" => Ok(ObjectType::Commit),
            _ => Err(ObjectTypeError::Invalid(value.to_string())),
        }
    }
}

impl std::fmt::Display for ObjectType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}
