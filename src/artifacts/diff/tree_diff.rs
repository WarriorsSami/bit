//! Tree diffing algorithm
//!
//! This module implements efficient diffing between Git tree objects to detect
//! added, deleted, and modified files.
//!
//! ## Algorithm
//!
//! The diff algorithm:
//! 1. Loads two tree objects (old and new)
//! 2. Recursively compares their entries
//! 3. Detects changes by comparing object IDs
//! 4. Supports filtering by change type (A/D/M/R)
//! 5. Can filter by specific file paths
//!
//! ## Performance
//!
//! Uses BTreeMap for sorted traversal and efficient comparison.
//! Only loads and expands subtrees when necessary (lazy evaluation).

use crate::areas::database::Database;
use crate::artifacts::database::database_entry::DatabaseEntry;
use crate::artifacts::log::path_filter::PathFilter;
use crate::artifacts::objects::object::ObjectBox;
use crate::artifacts::objects::object_id::ObjectId;
use crate::artifacts::objects::tree::Tree;
use bitflags::bitflags;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

bitflags! {
    /// Filter flags for diff output
    ///
    /// Allows filtering the diff to show only specific types of changes.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
    pub struct DiffFilter: u32 {
        /// Show added files
        const ADDED = 0b0001;
        /// Show deleted files
        const DELETED = 0b0010;
        /// Show modified files
        const MODIFIED = 0b0100;
        /// Show renamed files (not yet implemented)
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

/// Type of change detected in a tree diff
///
/// Represents the three fundamental types of file changes:
/// - Added: File exists in new tree but not old
/// - Deleted: File exists in old tree but not new
/// - Modified: File exists in both but with different content
#[derive(Debug, Clone, PartialEq)]
pub enum TreeChangeType {
    /// File was added
    Added(DatabaseEntry),
    /// File was deleted
    Deleted(DatabaseEntry),
    /// File was modified
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

/// Set of changes detected between two trees
pub type ChangeSet = BTreeMap<PathBuf, TreeChangeType>;

/// Map of tree entries (name -> database entry)
pub type TreeEntryMap = BTreeMap<String, DatabaseEntry>;

/// Tree diff engine
///
/// Compares two tree objects and produces a changeset of added, deleted,
/// and modified files. Supports filtering by change type and file paths.
///
/// ## Usage
///
/// ```ignore
/// let mut diff = TreeDiff::new(database);
/// diff.compare_oids(old_tree_oid, new_tree_oid, &path_filter)?;
/// let changes = diff.change_set();
/// ```
#[derive(Debug, Clone)]
pub struct TreeDiff<'r> {
    /// Reference to the object database
    database: &'r Database,
    /// Detected changes between trees
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
        path_filter: &PathFilter,
    ) -> anyhow::Result<()> {
        if old == new {
            return Ok(());
        }

        let old_tree_entries = self.inflate_oid_to_tree_entries(old)?;
        let new_tree_entries = self.inflate_oid_to_tree_entries(new)?;

        self.detect_deletions(&old_tree_entries, &new_tree_entries, path_filter)?;
        self.detect_additions(&old_tree_entries, &new_tree_entries, path_filter)?;

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

    fn detect_deletions(
        &mut self,
        old: &TreeEntryMap,
        new: &TreeEntryMap,
        path_filter: &PathFilter,
    ) -> anyhow::Result<()> {
        for (name, entry) in path_filter.filter_matching_entries(old.iter()) {
            let subpath_filter = path_filter.join_subpath_filter(name);
            let path = subpath_filter.path().to_path_buf();
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

            self.compare_oids(tree_a_oid, tree_b_oid, &subpath_filter)?;

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
        path_filter: &PathFilter,
    ) -> anyhow::Result<()> {
        for (name, entry) in path_filter.filter_matching_entries(new.iter()) {
            let subpath_filter = path_filter.join_subpath_filter(name);
            let path = subpath_filter.path().to_path_buf();
            let other = old.get(name);

            if other.is_some() {
                continue;
            }

            if entry.is_tree() {
                self.compare_oids(None, Some(&entry.oid), &subpath_filter)?;
            } else {
                // This is a newly added blob file
                self.change_set
                    .insert(path, TreeChangeType::Added(entry.clone()));
            }
        }

        Ok(())
    }
}
