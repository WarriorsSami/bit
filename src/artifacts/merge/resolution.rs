use crate::areas::index::Index;
use crate::areas::repository::Repository;
use crate::artifacts::checkout::migration::Migration;
use crate::artifacts::diff::tree_diff::{ChangeSet, TreeChangeType, TreeDiff};
use crate::artifacts::index::entry_mode::EntryMode;
use crate::artifacts::index::index_entry::{IndexEntry, MergeStage};
use crate::artifacts::log::path_filter::PathFilter;
use crate::artifacts::merge::inputs::MergeInputs;
use crate::artifacts::objects::object_id::ObjectId;
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

pub struct MergeResolution<'r> {
    repository: &'r Repository,
    merge_inputs: &'r MergeInputs<'r>,
}

/// How a particular path was changed on one side relative to the base
enum SideChange {
    None,
    Added(ObjectId, EntryMode),
    Modified(ObjectId, EntryMode),
    Deleted,
}

impl SideChange {
    fn from_tree_change(change: Option<&TreeChangeType>) -> Self {
        match change {
            None => SideChange::None,
            Some(TreeChangeType::Added(e)) => SideChange::Added(e.oid.clone(), e.mode),
            Some(TreeChangeType::Modified { new, .. }) => {
                SideChange::Modified(new.oid.clone(), new.mode)
            }
            Some(TreeChangeType::Deleted(_)) => SideChange::Deleted,
        }
    }
}

impl<'r> MergeResolution<'r> {
    pub fn new(repository: &'r Repository, merge_inputs: &'r MergeInputs<'r>) -> Self {
        Self {
            repository,
            merge_inputs,
        }
    }

    /// Perform a three-way merge. Returns true if any conflicts were written.
    pub fn execute(&self, index: &mut Index, right_name: &str) -> anyhow::Result<bool> {
        let left_changes = self.repository.database().tree_diff(
            Some(self.merge_inputs.base_oid()),
            Some(self.merge_inputs.left_oid()),
            &PathFilter::empty(),
        )?;

        let right_changes = self.repository.database().tree_diff(
            Some(self.merge_inputs.base_oid()),
            Some(self.merge_inputs.right_oid()),
            &PathFilter::empty(),
        )?;

        // Union of all changed paths
        let all_paths: Vec<PathBuf> = {
            let mut paths = std::collections::BTreeSet::new();
            for p in left_changes.changes().keys() {
                paths.insert(p.clone());
            }
            for p in right_changes.changes().keys() {
                paths.insert(p.clone());
            }
            paths.into_iter().collect()
        };

        // Clean right-only changes that can be applied via Migration
        let mut clean_right_changeset: ChangeSet = BTreeMap::new();
        let mut has_conflicts = false;

        for path in &all_paths {
            let left = SideChange::from_tree_change(left_changes.changes().get(path));
            let right = SideChange::from_tree_change(right_changes.changes().get(path));

            match (&left, &right) {
                // Only right changed → clean, apply via migration
                (SideChange::None, _) => {
                    if let Some(change) = right_changes.changes().get(path) {
                        clean_right_changeset.insert(path.clone(), change.clone());
                    }
                }

                // Only left changed → already in workspace, nothing to do
                (_, SideChange::None) => {}

                // Same OID added/modified on both sides → idempotent, no conflict
                (SideChange::Added(l, _), SideChange::Added(r, _)) if l == r => {}
                (SideChange::Modified(l, _), SideChange::Modified(r, _)) if l == r => {}

                // Both deleted → apply right (delete)
                (SideChange::Deleted, SideChange::Deleted) => {
                    if let Some(change) = right_changes.changes().get(path) {
                        clean_right_changeset.insert(path.clone(), change.clone());
                    }
                }

                // CONFLICT: Add/Add — different content on both sides (no base)
                (SideChange::Added(l_oid, l_mode), SideChange::Added(r_oid, _r_mode)) => {
                    has_conflicts = true;
                    let mode = *l_mode;
                    self.write_conflict_entries(index, path, None, Some(l_oid), Some(r_oid), mode)?;
                    self.write_conflict_markers(
                        path,
                        &self.load_blob_content(l_oid)?,
                        &self.load_blob_content(r_oid)?,
                        right_name,
                    )?;
                }

                // CONFLICT: Content conflict — both modified to different content
                (SideChange::Modified(l_oid, l_mode), SideChange::Modified(r_oid, _r_mode)) => {
                    has_conflicts = true;
                    let mode = *l_mode;
                    let base_oid = right_changes
                        .changes()
                        .get(path)
                        .and_then(|c| c.old_entry())
                        .map(|e| e.oid.clone());
                    self.write_conflict_entries(
                        index,
                        path,
                        base_oid.as_ref(),
                        Some(l_oid),
                        Some(r_oid),
                        mode,
                    )?;
                    self.write_conflict_markers(
                        path,
                        &self.load_blob_content(l_oid)?,
                        &self.load_blob_content(r_oid)?,
                        right_name,
                    )?;
                }

                // CONFLICT: Modify/Delete — left modified, right deleted
                (
                    SideChange::Modified(l_oid, l_mode) | SideChange::Added(l_oid, l_mode),
                    SideChange::Deleted,
                ) => {
                    has_conflicts = true;
                    let mode = *l_mode;
                    let base_oid = right_changes
                        .changes()
                        .get(path)
                        .and_then(|c| c.old_entry())
                        .map(|e| e.oid.clone());
                    self.write_conflict_entries(
                        index,
                        path,
                        base_oid.as_ref(),
                        Some(l_oid),
                        None,
                        mode,
                    )?;
                    // Keep our modified version in workspace (do not delete)
                }

                // CONFLICT: Delete/Modify — left deleted, right modified
                (
                    SideChange::Deleted,
                    SideChange::Modified(r_oid, r_mode) | SideChange::Added(r_oid, r_mode),
                ) => {
                    has_conflicts = true;
                    let mode = *r_mode;
                    let base_oid = right_changes
                        .changes()
                        .get(path)
                        .and_then(|c| c.old_entry())
                        .map(|e| e.oid.clone());
                    self.write_conflict_entries(
                        index,
                        path,
                        base_oid.as_ref(),
                        None,
                        Some(r_oid),
                        mode,
                    )?;
                    // Write their version to workspace
                    let content = self.load_blob_content(r_oid)?;
                    self.repository
                        .workspace()
                        .write_file(path, content.as_bytes())?;
                }

                // Remaining cross-combinations (e.g. Added+Modified) treated as conflicts
                (
                    SideChange::Added(l_oid, l_mode) | SideChange::Modified(l_oid, l_mode),
                    SideChange::Modified(r_oid, _) | SideChange::Added(r_oid, _),
                ) => {
                    has_conflicts = true;
                    let mode = *l_mode;
                    let base_oid = right_changes
                        .changes()
                        .get(path)
                        .and_then(|c| c.old_entry())
                        .map(|e| e.oid.clone());
                    self.write_conflict_entries(
                        index,
                        path,
                        base_oid.as_ref(),
                        Some(l_oid),
                        Some(r_oid),
                        mode,
                    )?;
                    self.write_conflict_markers(
                        path,
                        &self.load_blob_content(l_oid)?,
                        &self.load_blob_content(r_oid)?,
                        right_name,
                    )?;
                }
            }
        }

        // File/directory collision: right side adds files under a path our side has as a
        // tracked regular file. Rename our file to <path>~HEAD to preserve it, then let
        // Migration create the directory. Stage-2 entries are written after Migration so
        // Migration's discard_conflicts doesn't evict them.
        let mut file_dir_renames: Vec<(PathBuf, ObjectId, EntryMode)> = vec![];
        {
            let paths_to_check: Vec<PathBuf> = clean_right_changeset.keys().cloned().collect();
            let mut seen = BTreeSet::new();
            for right_path in &paths_to_check {
                for ancestor in right_path.ancestors().skip(1) {
                    if ancestor.as_os_str().is_empty() {
                        break;
                    }
                    let ancestor = ancestor.to_path_buf();
                    if !seen.contains(&ancestor)
                        && let Some(entry) = index.entry_by_path(&ancestor)
                    {
                        let oid = entry.oid.clone();
                        let mode = entry.metadata.mode;
                        let ws = self.repository.workspace().path();
                        let mut new_name = ancestor.as_os_str().to_owned();
                        new_name.push("~HEAD");
                        std::fs::rename(ws.join(&ancestor), ws.join(PathBuf::from(new_name)))?;
                        has_conflicts = true;
                        file_dir_renames.push((ancestor.clone(), oid, mode));
                        seen.insert(ancestor);
                    }
                }
            }
        }

        // Apply all clean right-side changes via Migration
        if !clean_right_changeset.is_empty() {
            let clean_diff =
                TreeDiff::from_changeset(self.repository.database(), clean_right_changeset);
            let mut migration = Migration::new_for_merge(self.repository, index, clean_diff);
            migration.apply_changes()?;
        }

        // Write stage-2 conflict entries for file/directory collisions (after Migration so
        // Migration's index.add doesn't evict them via discard_conflicts on parent paths).
        for (path, oid, mode) in file_dir_renames {
            index.add_conflict_entries(vec![IndexEntry::for_conflict(
                path,
                oid,
                mode,
                MergeStage::Ours,
            )])?;
        }

        Ok(has_conflicts)
    }

    fn write_conflict_entries(
        &self,
        index: &mut Index,
        path: &Path,
        base_oid: Option<&ObjectId>,
        ours_oid: Option<&ObjectId>,
        theirs_oid: Option<&ObjectId>,
        mode: EntryMode,
    ) -> anyhow::Result<()> {
        let mut entries = vec![];
        if let Some(oid) = base_oid {
            entries.push(IndexEntry::for_conflict(
                path.to_path_buf(),
                oid.clone(),
                mode,
                MergeStage::Base,
            ));
        }
        if let Some(oid) = ours_oid {
            entries.push(IndexEntry::for_conflict(
                path.to_path_buf(),
                oid.clone(),
                mode,
                MergeStage::Ours,
            ));
        }
        if let Some(oid) = theirs_oid {
            entries.push(IndexEntry::for_conflict(
                path.to_path_buf(),
                oid.clone(),
                mode,
                MergeStage::Theirs,
            ));
        }
        index.add_conflict_entries(entries)
    }

    fn write_conflict_markers(
        &self,
        path: &Path,
        ours_content: &str,
        theirs_content: &str,
        right_name: &str,
    ) -> anyhow::Result<()> {
        let content = format!(
            "<<<<<<< HEAD\n{}=======\n{}>>>>>>> {}\n",
            ours_content, theirs_content, right_name
        );
        self.repository
            .workspace()
            .write_file(path, content.as_bytes())
    }

    fn load_blob_content(&self, oid: &ObjectId) -> anyhow::Result<String> {
        self.repository
            .database()
            .parse_object_as_blob(oid)?
            .map(|b| b.content().to_string())
            .ok_or_else(|| anyhow::anyhow!("Expected blob at {}", oid))
    }
}
