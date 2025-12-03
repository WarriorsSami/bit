use crate::artifacts::index::entry_mode::EntryMode;
use crate::artifacts::objects::object_id::ObjectId;
use derive_new::new;

#[derive(Debug, Clone, PartialEq, new)]
pub struct DatabaseEntry {
    pub oid: ObjectId,
    pub mode: EntryMode,
}

impl DatabaseEntry {
    pub fn is_tree(&self) -> bool {
        self.mode.is_tree()
    }
}
