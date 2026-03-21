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

/// The kind of merge conflict on a path — drives workspace actions in `write_untracked_files`
enum ConflictKind {
    /// Both sides have different content; conflict markers are written to the file
    Content,
    /// Our side modified/added, their side deleted; our version is kept as-is
    ModifyDelete,
    /// Our side deleted, their side modified/added; their version is written to the workspace
    DeleteModify,
    /// Our side has a regular file where their side introduces a directory;
    /// the file is renamed to `<path>~HEAD` before migration runs
    FileDirectory,
}

/// All state needed to record and resolve one conflicted path
struct Conflict {
    path: PathBuf,
    base_oid: Option<ObjectId>,
    ours_oid: Option<ObjectId>,
    theirs_oid: Option<ObjectId>,
    mode: EntryMode,
    kind: ConflictKind,
}

impl<'r> MergeResolution<'r> {
    pub fn new(repository: &'r Repository, merge_inputs: &'r MergeInputs<'r>) -> Self {
        Self {
            repository,
            merge_inputs,
        }
    }

    pub fn execute(&self, index: &mut Index, right_name: &str) -> anyhow::Result<()> {
        let (clean_diff, conflicts) = self.prepare_tree_diffs(index)?;
        self.rename_file_directory_collisions(&conflicts)?;

        let diff = TreeDiff::from_changeset(self.repository.database(), clean_diff);
        let mut migration = Migration::new_for_merge(self.repository, index, diff);
        migration.apply_changes()?;

        self.add_conflicts_to_index(index, &conflicts)?;
        self.write_conflict_workspace_files(&conflicts, right_name)?;

        Ok(())
    }

    /// Compute both tree diffs, classify every changed path as clean or conflicted,
    /// and rename any workspace file that collides with an incoming directory.
    /// Returns the clean changeset (for Migration) and the full conflict list.
    fn prepare_tree_diffs(&self, index: &Index) -> anyhow::Result<(ChangeSet, Vec<Conflict>)> {
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

        let all_paths: BTreeSet<PathBuf> = left_changes
            .changes()
            .keys()
            .chain(right_changes.changes().keys())
            .cloned()
            .collect();

        let mut clean_diff: ChangeSet = BTreeMap::new();
        let mut conflicts: Vec<Conflict> = Vec::new();

        for path in &all_paths {
            let left = SideChange::from_tree_change(left_changes.changes().get(path));
            let right = SideChange::from_tree_change(right_changes.changes().get(path));

            let base_oid = || {
                right_changes
                    .changes()
                    .get(path)
                    .and_then(|c| c.old_entry())
                    .map(|e| e.oid.clone())
            };

            match (&left, &right) {
                // Only right changed → clean, apply via migration
                (SideChange::None, _) => {
                    if let Some(change) = right_changes.changes().get(path) {
                        clean_diff.insert(path.clone(), change.clone());
                    }
                }

                // Only left changed → already in workspace, nothing to do
                (_, SideChange::None) => {}

                // Same content on both sides → idempotent, no conflict
                (SideChange::Added(l, _), SideChange::Added(r, _)) if l == r => {}
                (SideChange::Modified(l, _), SideChange::Modified(r, _)) if l == r => {}

                // Both deleted → clean delete
                (SideChange::Deleted, SideChange::Deleted) => {
                    if let Some(change) = right_changes.changes().get(path) {
                        clean_diff.insert(path.clone(), change.clone());
                    }
                }

                // CONFLICT: Add/Add — different content, no common base
                (SideChange::Added(l_oid, l_mode), SideChange::Added(r_oid, _)) => {
                    conflicts.push(Conflict {
                        path: path.clone(),
                        base_oid: None,
                        ours_oid: Some(l_oid.clone()),
                        theirs_oid: Some(r_oid.clone()),
                        mode: *l_mode,
                        kind: ConflictKind::Content,
                    });
                }

                // CONFLICT: Both modified to different content
                (SideChange::Modified(l_oid, l_mode), SideChange::Modified(r_oid, _)) => {
                    conflicts.push(Conflict {
                        path: path.clone(),
                        base_oid: base_oid(),
                        ours_oid: Some(l_oid.clone()),
                        theirs_oid: Some(r_oid.clone()),
                        mode: *l_mode,
                        kind: ConflictKind::Content,
                    });
                }

                // CONFLICT: Our side modified/added, their side deleted
                (
                    SideChange::Modified(l_oid, l_mode) | SideChange::Added(l_oid, l_mode),
                    SideChange::Deleted,
                ) => {
                    conflicts.push(Conflict {
                        path: path.clone(),
                        base_oid: base_oid(),
                        ours_oid: Some(l_oid.clone()),
                        theirs_oid: None,
                        mode: *l_mode,
                        kind: ConflictKind::ModifyDelete,
                    });
                }

                // CONFLICT: Our side deleted, their side modified/added
                (
                    SideChange::Deleted,
                    SideChange::Modified(r_oid, r_mode) | SideChange::Added(r_oid, r_mode),
                ) => {
                    conflicts.push(Conflict {
                        path: path.clone(),
                        base_oid: base_oid(),
                        ours_oid: None,
                        theirs_oid: Some(r_oid.clone()),
                        mode: *r_mode,
                        kind: ConflictKind::DeleteModify,
                    });
                }

                // Remaining cross-combinations (e.g. Added vs Modified)
                (
                    SideChange::Added(l_oid, l_mode) | SideChange::Modified(l_oid, l_mode),
                    SideChange::Modified(r_oid, _) | SideChange::Added(r_oid, _),
                ) => {
                    conflicts.push(Conflict {
                        path: path.clone(),
                        base_oid: base_oid(),
                        ours_oid: Some(l_oid.clone()),
                        theirs_oid: Some(r_oid.clone()),
                        mode: *l_mode,
                        kind: ConflictKind::Content,
                    });
                }
            }
        }

        self.detect_file_directory_collisions(index, &clean_diff, &mut conflicts);

        Ok((clean_diff, conflicts))
    }

    /// Detect file/directory collisions: right side adds entries under a path that our
    /// side tracks as a regular file. Records a `ConflictKind::FileDirectory` entry for
    /// each such path; the actual rename is performed by `rename_file_directory_collisions`.
    fn detect_file_directory_collisions(
        &self,
        index: &Index,
        clean_diff: &ChangeSet,
        conflicts: &mut Vec<Conflict>,
    ) {
        let mut seen = BTreeSet::new();
        for right_path in clean_diff.keys() {
            for ancestor in right_path.ancestors().skip(1) {
                if ancestor.as_os_str().is_empty() {
                    break;
                }
                let ancestor = ancestor.to_path_buf();
                if !seen.contains(&ancestor)
                    && let Some(entry) = index.entry_by_path(&ancestor)
                {
                    conflicts.push(Conflict {
                        path: ancestor.clone(),
                        base_oid: None,
                        ours_oid: Some(entry.oid.clone()),
                        theirs_oid: None,
                        mode: entry.metadata.mode,
                        kind: ConflictKind::FileDirectory,
                    });
                    seen.insert(ancestor);
                }
            }
        }
    }

    /// Rename each file that collides with an incoming directory to `<name>~HEAD`,
    /// so that Migration can create a directory at that path.
    fn rename_file_directory_collisions(&self, conflicts: &[Conflict]) -> anyhow::Result<()> {
        for conflict in conflicts {
            if matches!(conflict.kind, ConflictKind::FileDirectory)
                && let Some(old_name) = conflict.path.file_name()
            {
                let new_name = format!("{}~HEAD", old_name.to_string_lossy());
                self.repository
                    .workspace()
                    .rename_file(&conflict.path, &PathBuf::from(new_name))?;
            }
        }
        Ok(())
    }

    /// Write index stage entries (1/2/3) for all conflicted paths.
    fn add_conflicts_to_index(
        &self,
        index: &mut Index,
        conflicts: &[Conflict],
    ) -> anyhow::Result<()> {
        for conflict in conflicts {
            let mut entries = vec![];
            if let Some(oid) = &conflict.base_oid {
                entries.push(IndexEntry::for_conflict(
                    conflict.path.clone(),
                    oid.clone(),
                    conflict.mode,
                    MergeStage::Base,
                ));
            }
            if let Some(oid) = &conflict.ours_oid {
                entries.push(IndexEntry::for_conflict(
                    conflict.path.clone(),
                    oid.clone(),
                    conflict.mode,
                    MergeStage::Ours,
                ));
            }
            if let Some(oid) = &conflict.theirs_oid {
                entries.push(IndexEntry::for_conflict(
                    conflict.path.clone(),
                    oid.clone(),
                    conflict.mode,
                    MergeStage::Theirs,
                ));
            }
            index.add_conflict_entries(entries)?;
        }
        Ok(())
    }

    /// Write workspace files for conflicts: conflict markers for content conflicts,
    /// their version for delete/modify conflicts. File/directory renames were already
    /// applied by `rename_file_directory_collisions`; modify/delete keeps our version as-is.
    fn write_conflict_workspace_files(
        &self,
        conflicts: &[Conflict],
        right_name: &str,
    ) -> anyhow::Result<()> {
        for conflict in conflicts {
            match conflict.kind {
                ConflictKind::Content => {
                    let ours = self.load_blob_content(conflict.ours_oid.as_ref().unwrap())?;
                    let theirs = self.load_blob_content(conflict.theirs_oid.as_ref().unwrap())?;
                    self.write_conflict_markers(&conflict.path, &ours, &theirs, right_name)?;
                }
                ConflictKind::DeleteModify => {
                    let content = self.load_blob_content(conflict.theirs_oid.as_ref().unwrap())?;
                    self.repository
                        .workspace()
                        .write_file(&conflict.path, content.as_bytes())?;
                }
                ConflictKind::ModifyDelete | ConflictKind::FileDirectory => {}
            }
        }
        Ok(())
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
