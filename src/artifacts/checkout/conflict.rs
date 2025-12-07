use crate::artifacts::database::database_entry::DatabaseEntry;
use crate::artifacts::index::index_entry::{EntryMetadata, IndexEntry};

#[derive(Debug)]
pub struct ConflictMessage {
    pub header: &'static str,
    pub footer: &'static str,
}

impl From<&ConflictType> for ConflictMessage {
    fn from(value: &ConflictType) -> Self {
        match value {
            ConflictType::StaleFile => Self {
                header: "Your local changes to the following files would be overwritten by checkout:",
                footer: "Please commit your changes or stash them before you switch branches.",
            },
            ConflictType::StaleDirectory => Self {
                header: "Updating the following directories would lose untracked files in them:",
                footer: "\n",
            },
            ConflictType::UntrackedOverwritten => Self {
                header: "The following untracked working tree files would be overwritten by checkout:",
                footer: "Please move or remove them before you switch branches.",
            },
            ConflictType::UntrackedRemoved => Self {
                header: "The following untracked working tree files would be removed by checkout:",
                footer: "Please move or remove them before you switch branches.",
            },
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ConflictType {
    StaleFile,
    StaleDirectory,
    UntrackedOverwritten,
    UntrackedRemoved,
}

impl ConflictType {
    pub fn get_conflict_type(
        stat: Option<&EntryMetadata>,
        entry: Option<&IndexEntry>,
        new_entry: Option<&DatabaseEntry>,
    ) -> ConflictType {
        if entry.is_some() {
            ConflictType::StaleFile
        } else if let Some(stat) = stat
            && stat.mode.is_tree()
        {
            ConflictType::StaleDirectory
        } else if new_entry.is_some() {
            ConflictType::UntrackedOverwritten
        } else {
            ConflictType::UntrackedRemoved
        }
    }
}
