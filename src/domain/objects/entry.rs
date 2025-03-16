#[derive(Debug, Clone)]
pub enum EntryMode {
    Regular,
    Executable,
    Directory,
}

impl EntryMode {
    pub fn as_str(&self) -> &str {
        match self {
            EntryMode::Regular => "100644",
            EntryMode::Executable => "100755",
            EntryMode::Directory => "40000",
        }
    }
}

impl TryFrom<&str> for EntryMode {
    type Error = anyhow::Error;

    fn try_from(value: &str) -> anyhow::Result<Self> {
        match value {
            "100644" => Ok(EntryMode::Regular),
            "100755" => Ok(EntryMode::Executable),
            "40000" => Ok(EntryMode::Directory),
            _ => Err(anyhow::anyhow!("Invalid entry mode")),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Entry {
    pub name: String,
    pub oid: String,
    pub mode: EntryMode,
}

impl Entry {
    pub fn new(name: String, oid: String, mode: EntryMode) -> Self {
        Self { name, oid, mode }
    }
}
