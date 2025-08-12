#[derive(Debug, Clone, Eq, Ord, Default, PartialEq, PartialOrd)]
pub enum FileMode {
    #[default]
    Regular,
    Executable,
}

#[derive(Debug, Clone, Eq, Ord, Default, PartialEq, PartialOrd)]
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
}

impl From<u32> for EntryMode {
    fn from(mode: u32) -> Self {
        match mode {
            0o100644 => EntryMode::File(FileMode::Regular),
            0o100755 => EntryMode::File(FileMode::Executable),
            0o40000 => EntryMode::Directory,
            _ => panic!("Invalid entry mode"),
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
    type Error = anyhow::Error;

    fn try_from(value: EntryMode) -> anyhow::Result<Self> {
        match value {
            EntryMode::File(FileMode::Regular) => Ok(FileMode::Regular),
            EntryMode::File(FileMode::Executable) => Ok(FileMode::Executable),
            _ => Err(anyhow::anyhow!("Invalid entry mode")),
        }
    }
}

impl TryFrom<&str> for EntryMode {
    type Error = anyhow::Error;

    fn try_from(value: &str) -> anyhow::Result<Self> {
        match value {
            "100644" => Ok(EntryMode::File(FileMode::Regular)),
            "100755" => Ok(EntryMode::File(FileMode::Executable)),
            "40000" => Ok(EntryMode::Directory),
            _ => Err(anyhow::anyhow!("Invalid entry mode")),
        }
    }
}
