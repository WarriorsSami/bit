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
    /// Our side has a regular file at `path`; theirs introduces a directory there.
    /// Our file is renamed to `rename` (`<path>~HEAD`) before Migration runs.
    FileDirectory,
    /// Our side has a directory at `path`; theirs introduces a regular file there.
    /// Their file is written to `rename` (`<path>~<right_name>`) before Migration runs;
    /// `path` is removed from `clean_diff` so Migration never touches the directory.
    DirectoryFile,
}

/// All states needed to record and resolve one conflicted path
struct Conflict {
    path: PathBuf,
    base_oid: Option<ObjectId>,
    ours_oid: Option<ObjectId>,
    theirs_oid: Option<ObjectId>,
    mode: EntryMode,
    kind: ConflictKind,
    /// Rename target for path-type collision conflicts:
    /// - `FileDirectory`: new name for our file (`<path>~HEAD`)
    /// - `DirectoryFile`: destination for their file (`<path>~<right_name>`)
    rename: Option<PathBuf>,
}

impl From<&Conflict> for &'static str {
    fn from(conflict: &Conflict) -> Self {
        match conflict.kind {
            ConflictKind::Content if conflict.base_oid.is_some() => "content",
            ConflictKind::Content => "add/add",
            ConflictKind::ModifyDelete => "modify/delete",
            ConflictKind::DeleteModify => "delete/modify",
            ConflictKind::FileDirectory => "file/directory",
            ConflictKind::DirectoryFile => "directory/file",
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

    fn log(&self, message: &str) -> anyhow::Result<()> {
        writeln!(self.repository.writer(), "{}", message)?;

        Ok(())
    }

    fn log_branch_names(&self, conflict: &Conflict) -> (String, String) {
        let (left, right) = (
            self.merge_inputs.left_name(),
            self.merge_inputs.right_name(),
        );
        if conflict.theirs_oid.is_some() {
            (left.to_string(), right.to_string())
        } else {
            (right.to_string(), left.to_string())
        }
    }

    fn log_left_right_conflict(&self, conflict: &Conflict) -> anyhow::Result<()> {
        let conflict_type: &str = conflict.into();

        self.log(&format!(
            "CONFLICT ({conflict_type}): Merge conflict in {}",
            conflict.path.display()
        ))?;

        Ok(())
    }

    fn log_modify_delete_conflict(&self, conflict: &Conflict) -> anyhow::Result<()> {
        let conflict_type: &str = conflict.into();

        let path = &conflict.path.display();
        let (deleted, modified) = self.log_branch_names(conflict);
        let rename = match conflict.rename.as_ref() {
            Some(r) => format!(" at {}", r.display()),
            None => String::new(),
        };

        self.log(&format!(
            "CONFLICT ({conflict_type}): {path} deleted in {deleted} and modified in {modified}.\
            Version from {modified} of {path} left in tree{rename}",
        ))?;

        Ok(())
    }

    fn log_file_directory_conflict(&self, conflict: &Conflict) -> anyhow::Result<()> {
        let conflict_type: &str = conflict.into();

        let path = &conflict.path.display();
        let (branch, _) = self.log_branch_names(conflict);
        let rename = match conflict.rename.as_ref() {
            Some(r) => format!("{}", r.display()),
            None => String::new(),
        };

        self.log(&format!(
            "CONFLICT ({conflict_type}): There is a directory with name {path} in {branch}. Adding {path} as {rename}",
        ))?;

        Ok(())
    }

    fn log_conflict(&self, conflict: &Conflict) -> anyhow::Result<()> {
        match conflict.kind {
            ConflictKind::Content => self.log_left_right_conflict(conflict),
            ConflictKind::ModifyDelete | ConflictKind::DeleteModify => {
                self.log_modify_delete_conflict(conflict)
            }
            ConflictKind::FileDirectory | ConflictKind::DirectoryFile => {
                self.log_file_directory_conflict(conflict)
            }
        }
    }

    /// Run the full three-way merge against the current index and workspace.
    ///
    /// Steps and ordering constraints:
    ///
    /// 1. **`prepare_tree_diffs`** — diff base→left and base→right, then classify every
    ///    changed path as either clean (only one side touched it) or conflicted (both sides
    ///    touched it differently). Must run first because every subsequent step consumes
    ///    its output.
    ///
    /// 2. **`rename_file_directory_collisions`** — for every `FileDirectory` conflict,
    ///    rename the tracked file at that path to `<name>~HEAD` in the workspace. Must
    ///    run *before* Migration because Migration will try to create a real directory at
    ///    the collided path; that fails on any normal filesystem if a regular file is
    ///    already sitting there.
    ///
    /// 3. **`migration.apply_changes`** — write the clean diff to the workspace and index
    ///    (creates/updates/deletes files and directories). Must run before
    ///    `add_conflicts_to_index` because Migration calls `index.add`, which internally
    ///    evicts any existing conflict entries (stages 1–3) for a path when it promotes
    ///    it to stage 0. Writing conflict entries first would therefore be lost.
    ///
    /// 4. **`add_conflicts_to_index`** — write stage-1/2/3 index entries for every
    ///    conflicted path. Must run after Migration for the eviction reason above, and
    ///    before `write_conflict_workspace_files` so that the index accurately reflects
    ///    the conflict state before any workspace writes touch the same paths.
    ///
    /// 5. **`write_conflict_workspace_files`** — write the on-disk representation of each
    ///    conflict (markers for content conflicts, their blob for delete/modify). Runs last
    ///    because it only needs the already-computed conflict list and does not affect the
    ///    index.
    pub fn execute(&self, index: &mut Index, right_name: &str) -> anyhow::Result<()> {
        let (clean_diff, conflicts) = self.prepare_tree_diffs(index, right_name)?;
        self.rename_file_directory_collisions(&conflicts)?;

        let diff = TreeDiff::from_changeset(self.repository.database(), clean_diff);
        let mut migration = Migration::new_for_merge(self.repository, index, diff);
        migration.apply_changes()?;

        self.add_conflicts_to_index(index, &conflicts)?;
        self.write_conflict_workspace_files(&conflicts, right_name)?;

        Ok(())
    }

    /// Compute base→left and base→right tree diffs, then classify every path that
    /// appears in either diff into one of the scenarios below.
    ///
    /// Returns the clean changeset (handed to Migration) and the full conflict list.
    ///
    /// ## Classification table
    ///
    /// | Left (ours)             | Right (theirs)           | Outcome                                                   | Resolution                                                                 |
    /// |-------------------------|--------------------------|-----------------------------------------------------------|----------------------------------------------------------------------------|
    /// | None                    | any                      | **Clean** — only right touched it                         | Migration writes/updates/deletes the file; index promoted to stage 0       |
    /// | any                     | None                     | **Clean** — only left touched it                          | Already in workspace and index; nothing to do                              |
    /// | Added(A)                | Added(A)                 | **Clean** — identical blob added on both sides            | No-op; both sides already agree                                            |
    /// | Modified(A)             | Modified(A)              | **Clean** — both converged to the same blob               | No-op; both sides already agree                                            |
    /// | Deleted                 | Deleted                  | **Clean** — both deleted                                  | Migration applies the delete; index entry removed                          |
    /// | Added(A)                | Added(B)                 | **Conflict** `Content` — different blobs, no common base  | Conflict markers written to workspace; stages 2 (ours) + 3 (theirs)       |
    /// | Modified(A)             | Modified(B)              | **Conflict** `Content` — diverged from a shared base      | Conflict markers written to workspace; stages 1 (base) + 2 + 3            |
    /// | Modified/Added          | Deleted                  | **Conflict** `ModifyDelete` — we kept it, they removed it | Our version left in workspace as-is; stages 1 (base) + 2 (ours)           |
    /// | Deleted                 | Modified/Added           | **Conflict** `DeleteModify` — we removed it, they kept it | Their blob written to workspace; stages 1 (base) + 3 (theirs)             |
    /// | Added/Modified(A)       | Modified/Added(B)        | **Conflict** `Content` — remaining cross-combinations     | Conflict markers written to workspace; stages 1 (base, if any) + 2 + 3    |
    /// | file at ancestor path   | entries added beneath it | **Conflict** `FileDirectory` — dir/file path collision    | Our file renamed to `<name>~HEAD`; Migration creates the directory; stage 2 (ours) |
    fn prepare_tree_diffs(
        &self,
        index: &Index,
        right_name: &str,
    ) -> anyhow::Result<(ChangeSet, Vec<Conflict>)> {
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

        enum MergeTriviality {
            None,
            Trivial,
            Conflict,
        }

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

            let merge_triviality = match (&left, &right) {
                // Only right changed → clean, apply via migration
                (SideChange::None, _) => {
                    if let Some(change) = right_changes.changes().get(path) {
                        clean_diff.insert(path.clone(), change.clone());
                    }
                    MergeTriviality::None
                }

                // Only left changed → already in workspace, nothing to do
                (_, SideChange::None) => MergeTriviality::None,

                // Same content on both sides → idempotent, no conflict
                (SideChange::Added(l, _), SideChange::Added(r, _))
                | (SideChange::Modified(l, _), SideChange::Modified(r, _))
                    if l == r =>
                {
                    MergeTriviality::Trivial
                }

                // Both deleted → clean delete
                (SideChange::Deleted, SideChange::Deleted) => {
                    if let Some(change) = right_changes.changes().get(path) {
                        clean_diff.insert(path.clone(), change.clone());
                    }
                    MergeTriviality::Trivial
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
                        rename: None,
                    });
                    MergeTriviality::Conflict
                }

                // CONFLICT: Both modified to different content
                // Or remaining cross-combinations (e.g. Added vs Modified)
                (SideChange::Modified(l_oid, l_mode), SideChange::Modified(r_oid, _))
                | (
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
                        rename: None,
                    });
                    MergeTriviality::Conflict
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
                        rename: None,
                    });
                    MergeTriviality::Conflict
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
                        rename: None,
                    });
                    MergeTriviality::Conflict
                }
            };

            // report trivial merges and conflicts as we classify them;
            // this way the user gets immediate feedback on all cleanly merged paths
            // instead of waiting until the end to see the full list of conflicts
            match merge_triviality {
                MergeTriviality::None => {}
                MergeTriviality::Trivial => {
                    self.log(&format!("Auto-merging {}", path.display()))?;
                }
                MergeTriviality::Conflict => {
                    self.log(&format!("Auto-merging {} failed", path.display()))?;
                    self.log_conflict(
                        conflicts
                            .last()
                            .expect("Just pushed a conflict, must exist"),
                    )?;
                }
            }
        }

        self.detect_file_directory_collisions(index, &mut clean_diff, &mut conflicts, right_name)?;

        Ok((clean_diff, conflicts))
    }

    /// Detect both directions of file/directory path-type collisions:
    ///
    /// - **file/directory**: right side adds entries *under* a path that our side tracks
    ///   as a regular file. Our file is renamed to `<name>~HEAD` before Migration runs.
    /// - **directory/file**: right side adds a file at a path where our side has directory
    ///   entries. Their file is written to `<name>~<right_name>` and the path is removed
    ///   from `clean_diff` so Migration never attempts to overwrite the directory.
    fn detect_file_directory_collisions(
        &self,
        index: &Index,
        clean_diff: &mut ChangeSet,
        conflicts: &mut Vec<Conflict>,
        right_name: &str,
    ) -> anyhow::Result<()> {
        // file/directory: right adds entries beneath a path we track as a file
        let mut seen_file: BTreeSet<PathBuf> = BTreeSet::new();
        for right_path in clean_diff.keys() {
            for ancestor in right_path.ancestors().skip(1) {
                if ancestor.as_os_str().is_empty() {
                    break;
                }
                let ancestor = ancestor.to_path_buf();
                if !seen_file.contains(&ancestor)
                    && let Some(entry) = index.entry_by_path(&ancestor)
                {
                    conflicts.push(Conflict {
                        path: ancestor.clone(),
                        base_oid: None,
                        ours_oid: Some(entry.oid.clone()),
                        theirs_oid: None,
                        mode: entry.metadata.mode,
                        kind: ConflictKind::FileDirectory,
                        rename: Some({
                            let name = format!(
                                "{}~HEAD",
                                ancestor.file_name().unwrap_or_default().to_string_lossy()
                            );
                            ancestor.with_file_name(name)
                        }),
                    });
                    seen_file.insert(ancestor.clone());

                    self.log(&format!("Adding {}", ancestor.display()))?;
                    self.log_conflict(
                        conflicts
                            .last()
                            .expect("Just pushed a conflict, must exist"),
                    )?;
                }
            }
        }

        // directory/file: right adds a file at a path that is a directory in our tree
        let mut seen_dir: BTreeSet<PathBuf> = BTreeSet::new();
        let candidate_paths: Vec<PathBuf> = clean_diff.keys().cloned().collect();
        for right_path in candidate_paths {
            if seen_dir.contains(&right_path) {
                continue;
            }
            let has_children = index
                .entries_under_path(&right_path)
                .into_iter()
                .any(|p| p != right_path && p.starts_with(&right_path));
            if !has_children {
                continue;
            }
            let (incoming_oid, incoming_mode) = match clean_diff.get(&right_path) {
                Some(TreeChangeType::Added(e)) | Some(TreeChangeType::Modified { new: e, .. }) => {
                    (e.oid.clone(), e.mode)
                }
                _ => continue,
            };
            let file_name = right_path
                .file_name()
                .map(|n| format!("{}~{}", n.to_string_lossy(), right_name))
                .unwrap_or_default();
            let rename = right_path.with_file_name(file_name);
            conflicts.push(Conflict {
                path: right_path.clone(),
                base_oid: None,
                ours_oid: None,
                theirs_oid: Some(incoming_oid),
                mode: incoming_mode,
                kind: ConflictKind::DirectoryFile,
                rename: Some(rename),
            });
            seen_dir.insert(right_path.clone());

            self.log(&format!("Adding {}", right_path.display()))?;
            self.log_conflict(
                conflicts
                    .last()
                    .expect("Just pushed a conflict, must exist"),
            )?;
        }

        // Remove directory/file paths from clean_diff — Migration must not attempt to
        // write a file at a path that is already a directory in the workspace.
        for path in &seen_dir {
            clean_diff.remove(path);
        }

        Ok(())
    }

    /// Resolve workspace-level path-type collisions before Migration runs:
    ///
    /// - **file/directory** (`ours_oid` is Some): rename our existing file to `<name>~HEAD`
    ///   so Migration can create a directory at that path.
    /// - **directory/file** (`theirs_oid` is Some, `ours_oid` is None): write their blob
    ///   to `<name>~<right_name>` so the incoming file is preserved without overwriting
    ///   our directory.
    fn rename_file_directory_collisions(&self, conflicts: &[Conflict]) -> anyhow::Result<()> {
        for conflict in conflicts {
            match conflict.kind {
                ConflictKind::FileDirectory => {
                    // Rename our file to ~HEAD so Migration can create a directory there
                    let rename = conflict
                        .rename
                        .as_ref()
                        .expect("FileDirectory conflict must have a rename target");
                    self.repository
                        .workspace()
                        .rename_file(&conflict.path, rename)?;
                }
                ConflictKind::DirectoryFile => {
                    // Write their blob to ~<right_name>; Migration will not touch this path
                    let rename = conflict
                        .rename
                        .as_ref()
                        .expect("DirectoryFile conflict must have a rename target");
                    let theirs_oid = conflict
                        .theirs_oid
                        .as_ref()
                        .expect("DirectoryFile conflict must have theirs_oid");
                    let content = self.load_blob_content(theirs_oid)?;
                    self.repository
                        .workspace()
                        .write_file(rename, content.as_bytes())?;
                }
                _ => {}
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
                ConflictKind::ModifyDelete
                | ConflictKind::FileDirectory
                | ConflictKind::DirectoryFile => {}
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
