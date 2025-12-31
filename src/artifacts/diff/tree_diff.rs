use crate::areas::database::Database;
use crate::artifacts::database::database_entry::DatabaseEntry;
use crate::artifacts::objects::object::ObjectBox;
use crate::artifacts::objects::object_id::ObjectId;
use crate::artifacts::objects::tree::Tree;
use bitflags::bitflags;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
    pub struct DiffFilter: u32 {
        const ADDED = 0b0001;
        const DELETED = 0b0010;
        const MODIFIED = 0b0100;
        const RENAMED = 0b1000;
    }
}

impl DiffFilter {
    pub fn try_parse(s: &str) -> Option<Self> {
        let mut filter = Self::empty();

        for c in s.chars() {
            match c {
                'A' => filter |= Self::ADDED,
                'D' => filter |= Self::DELETED,
                'M' => filter |= Self::MODIFIED,
                'R' => filter |= Self::RENAMED,
                _ => return None,
            }
        }

        Some(filter)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum TreeChangeType {
    Added(DatabaseEntry),
    Deleted(DatabaseEntry),
    Modified {
        old: DatabaseEntry,
        new: DatabaseEntry,
    },
}

impl TreeChangeType {
    pub fn from_entries(old: Option<DatabaseEntry>, new: Option<DatabaseEntry>) -> Option<Self> {
        match (old, new) {
            (None, Some(new)) => Some(TreeChangeType::Added(new)),
            (Some(old), None) => Some(TreeChangeType::Deleted(old)),
            (Some(old), Some(new)) if old != new => Some(TreeChangeType::Modified { old, new }),
            _ => None, // No change or both are None
        }
    }

    pub fn matches_filter(&self, filter: DiffFilter) -> bool {
        match self {
            TreeChangeType::Added(_) => filter.contains(DiffFilter::ADDED),
            TreeChangeType::Deleted(_) => filter.contains(DiffFilter::DELETED),
            TreeChangeType::Modified { .. } => filter.contains(DiffFilter::MODIFIED),
        }
    }

    pub fn old_entry(&self) -> Option<&DatabaseEntry> {
        match self {
            TreeChangeType::Deleted(entry) => Some(entry),
            TreeChangeType::Modified { old, .. } => Some(old),
            TreeChangeType::Added(_) => None,
        }
    }

    pub fn new_entry(&self) -> Option<&DatabaseEntry> {
        match self {
            TreeChangeType::Added(entry) => Some(entry),
            TreeChangeType::Modified { new, .. } => Some(new),
            TreeChangeType::Deleted(_) => None,
        }
    }

    pub fn status_char(&self) -> char {
        match self {
            TreeChangeType::Added(_) => 'A',
            TreeChangeType::Deleted(_) => 'D',
            TreeChangeType::Modified { .. } => 'M',
        }
    }
}

pub type ChangeSet = BTreeMap<PathBuf, TreeChangeType>;
pub type TreeEntryMap = BTreeMap<String, DatabaseEntry>;

#[derive(Debug)]
pub struct TreeDiff<'r> {
    database: &'r Database,
    change_set: ChangeSet,
}

impl<'r> TreeDiff<'r> {
    pub fn new(database: &'r Database) -> Self {
        TreeDiff {
            database,
            change_set: BTreeMap::new(),
        }
    }

    pub fn changes(&self) -> &ChangeSet {
        &self.change_set
    }

    pub fn get_entries(&self, path: &Path) -> (Option<&DatabaseEntry>, Option<&DatabaseEntry>) {
        if let Some(change) = self.change_set.get(path) {
            (change.old_entry(), change.new_entry())
        } else {
            (None, None)
        }
    }

    pub fn compare_oids(
        &mut self,
        old: Option<&ObjectId>,
        new: Option<&ObjectId>,
        prefix: &Path,
    ) -> anyhow::Result<()> {
        if old == new {
            return Ok(());
        }

        let old_tree_entries = self.inflate_oid_to_tree_entries(old)?;
        let new_tree_entries = self.inflate_oid_to_tree_entries(new)?;

        self.detect_deletions(&old_tree_entries, &new_tree_entries, prefix)?;
        self.detect_additions(&old_tree_entries, &new_tree_entries, prefix)?;

        Ok(())
    }

    fn inflate_oid_to_tree_entries(&self, oid: Option<&ObjectId>) -> anyhow::Result<TreeEntryMap> {
        match oid {
            None => Ok(BTreeMap::new()),
            Some(oid) => Ok(self
                .inflate_oid_to_tree(oid)?
                .into_entries()
                .collect::<BTreeMap<_, _>>()),
        }
    }

    fn inflate_oid_to_tree(&'_ self, oid: &ObjectId) -> anyhow::Result<Tree<'_>> {
        let object = self.database.parse_object(oid)?;

        match object {
            ObjectBox::Tree(tree) => Ok(*tree),
            ObjectBox::Commit(commit) => {
                let tree_oid = commit.tree_oid();
                self.inflate_oid_to_tree(tree_oid)
            }
            _ => Err(anyhow::anyhow!("Invalid tree object {}", oid.to_string())),
        }
    }

    // TODO: optimize by removing redundant cloning
    fn detect_deletions(
        &mut self,
        old: &TreeEntryMap,
        new: &TreeEntryMap,
        prefix: &Path,
    ) -> anyhow::Result<()> {
        for (name, entry) in old {
            let path = prefix.join(name);
            let other = new.get(name);

            if let Some(other) = other
                && other == entry
            {
                continue;
            }

            let tree_a_oid = if entry.is_tree() {
                Some(&entry.oid)
            } else {
                None
            };
            let tree_b_oid = if let Some(other) = other
                && other.is_tree()
            {
                Some(&other.oid)
            } else {
                None
            };

            self.compare_oids(tree_a_oid, tree_b_oid, &path)?;

            let blob_a = if entry.is_tree() {
                None
            } else {
                Some(entry.clone())
            };
            let blob_b = match other {
                Some(other) if !other.is_tree() => Some(other.clone()),
                _ => None,
            };

            // Determine change type based on old and new entries
            if let Some(change_type) = TreeChangeType::from_entries(blob_a, blob_b) {
                self.change_set.insert(path, change_type);
            }
        }

        Ok(())
    }

    fn detect_additions(
        &mut self,
        old: &TreeEntryMap,
        new: &TreeEntryMap,
        prefix: &Path,
    ) -> anyhow::Result<()> {
        for (name, entry) in new {
            let path = prefix.join(name);
            let other = old.get(name);

            if other.is_some() {
                continue;
            }

            if entry.is_tree() {
                self.compare_oids(None, Some(&entry.oid), &path)?;
            } else {
                // This is a newly added blob file
                self.change_set
                    .insert(path, TreeChangeType::Added(entry.clone()));
            }
        }

        Ok(())
    }
}
