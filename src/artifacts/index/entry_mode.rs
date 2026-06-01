#[derive(Debug, thiserror::Error)]
pub enum EntryModeError {
    #[error("invalid entry mode string: {0}")]
    InvalidModeString(String),
    #[error("invalid entry mode value: {0:#o}")]
    InvalidModeValue(u32),
    #[error("cannot convert directory mode to file mode")]
    NotAFileMode,
}

#[derive(Debug, Copy, Clone, Eq, Ord, Default, PartialEq, PartialOrd)]
pub enum FileMode {
    #[default]
    Regular,
    Executable,
}

#[derive(Debug, Copy, Clone, Eq, Ord, Default, PartialEq, PartialOrd)]
pub enum EntryMode {
    File(FileMode),
    #[default]
    Directory,
}

impl EntryMode {
    pub fn as_str(&self) -> &str {
        match self {
            EntryMode::File(FileMode::Regular) => "100644",
            EntryMode::File(FileMode::Executable) => "100755",
            EntryMode::Directory => "40000",
        }
    }

    pub fn as_u32(&self) -> u32 {
        match self {
            EntryMode::File(FileMode::Regular) => 0o100644,
            EntryMode::File(FileMode::Executable) => 0o100755,
            EntryMode::Directory => 0o40000,
        }
    }

    pub fn from_octal_str(s: &str) -> Result<Self, EntryModeError> {
        match s {
            "100644" => Ok(EntryMode::File(FileMode::Regular)),
            "100755" => Ok(EntryMode::File(FileMode::Executable)),
            "40000" => Ok(EntryMode::Directory),
            _ => Err(EntryModeError::InvalidModeString(s.to_string())),
        }
    }

    pub fn is_tree(&self) -> bool {
        matches!(self, EntryMode::Directory)
    }
}

impl TryFrom<u32> for EntryMode {
    type Error = EntryModeError;

    fn try_from(mode: u32) -> Result<Self, EntryModeError> {
        match mode {
            0o100644 => Ok(EntryMode::File(FileMode::Regular)),
            0o100755 => Ok(EntryMode::File(FileMode::Executable)),
            0o40000 => Ok(EntryMode::Directory),
            _ => Err(EntryModeError::InvalidModeValue(mode)),
        }
    }
}

impl From<EntryMode> for u32 {
    fn from(mode: EntryMode) -> Self {
        match mode {
            EntryMode::File(FileMode::Regular) => 0o100644,
            EntryMode::File(FileMode::Executable) => 0o100755,
            EntryMode::Directory => 0o40000,
        }
    }
}

impl From<FileMode> for EntryMode {
    fn from(mode: FileMode) -> Self {
        EntryMode::File(mode)
    }
}

impl From<&FileMode> for &EntryMode {
    fn from(mode: &FileMode) -> Self {
        match mode {
            FileMode::Regular => &EntryMode::File(FileMode::Regular),
            FileMode::Executable => &EntryMode::File(FileMode::Executable),
        }
    }
}

impl TryFrom<EntryMode> for FileMode {
    type Error = EntryModeError;

    fn try_from(value: EntryMode) -> Result<Self, EntryModeError> {
        match value {
            EntryMode::File(FileMode::Regular) => Ok(FileMode::Regular),
            EntryMode::File(FileMode::Executable) => Ok(FileMode::Executable),
            _ => Err(EntryModeError::NotAFileMode),
        }
    }
}

impl TryFrom<&str> for EntryMode {
    type Error = EntryModeError;

    fn try_from(value: &str) -> Result<Self, EntryModeError> {
        match value {
            "100644" => Ok(EntryMode::File(FileMode::Regular)),
            "100755" => Ok(EntryMode::File(FileMode::Executable)),
            "40000" => Ok(EntryMode::Directory),
            _ => Err(EntryModeError::InvalidModeString(value.to_string())),
        }
    }
}
