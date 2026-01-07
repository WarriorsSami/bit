//! Database entry representation
//!
//! Database entries represent references to objects stored in the object database.
//! They contain the object ID and mode information, used when reading tree objects.

use crate::artifacts::index::entry_mode::EntryMode;
use crate::artifacts::objects::object_id::ObjectId;
use derive_new::new;

/// Entry from a tree object in the database
///
/// Contains the object ID and entry mode for a file or subtree.
/// Used when traversing tree objects during diff, checkout, etc.
#[derive(Debug, Clone, PartialEq, new)]
pub struct DatabaseEntry {
    /// Object ID (hash of the referenced object)
    pub oid: ObjectId,
    /// Entry mode (file permissions and type)
    pub mode: EntryMode,
}

impl DatabaseEntry {
    pub fn is_tree(&self) -> bool {
        self.mode.is_tree()
    }
}
