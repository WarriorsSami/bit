use crate::areas::index::Index;
use crate::areas::repository::Repository;
use crate::artifacts::checkout::conflict::{ConflictMessage, ConflictType};
use crate::artifacts::database::database_entry::DatabaseEntry;
use crate::artifacts::diff::tree_diff::{TreeChangeType, TreeDiff};
use crate::artifacts::index::index_entry::IndexEntry;
use crate::artifacts::objects::object_id::ObjectId;
use crate::artifacts::status::file_change::{IndexChangeType, WorkspaceChangeType};
use crate::artifacts::status::inspector::Inspector;
use anyhow::Context;
use std::collections::{BTreeSet, HashMap};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ActionType {
    Add,
    Delete,
    Modify,
}

pub type ActionsSet = HashMap<ActionType, Vec<(PathBuf, Option<DatabaseEntry>)>>;
pub type ConflictsSet = HashMap<ConflictType, Vec<PathBuf>>;

pub struct Migration<'r> {
    repository: &'r Repository,
    tree_diff: TreeDiff<'r>,
    index: &'r mut Index,
    inspector: Inspector<'r>,
    actions: ActionsSet,
    conflicts: ConflictsSet,
    mkdirs: BTreeSet<PathBuf>,
    rmdirs: BTreeSet<PathBuf>,
}

impl<'r> Migration<'r> {
    pub fn new(repository: &'r Repository, index: &'r mut Index, tree_diff: TreeDiff<'r>) -> Self {
        let actions = HashMap::from([
            (ActionType::Add, Vec::new()),
            (ActionType::Delete, Vec::new()),
            (ActionType::Modify, Vec::new()),
        ]);

        let conflicts = HashMap::from([
            (ConflictType::StaleFile, Vec::new()),
            (ConflictType::StaleDirectory, Vec::new()),
            (ConflictType::UntrackedOverwritten, Vec::new()),
            (ConflictType::UntrackedRemoved, Vec::new()),
        ]);

        let inspector = Inspector::new(repository);

        Self {
            repository,
            index,
            tree_diff,
            inspector,
            actions,
            conflicts,
            mkdirs: BTreeSet::new(),
            rmdirs: BTreeSet::new(),
        }
    }

    pub fn actions(&self) -> &ActionsSet {
        &self.actions
    }

    pub fn mkdirs(&self) -> &BTreeSet<PathBuf> {
        &self.mkdirs
    }

    pub fn rmdirs(&self) -> &BTreeSet<PathBuf> {
        &self.rmdirs
    }

    pub fn apply_changes(&mut self) -> anyhow::Result<()> {
        self.plan_changes()?;
        self.update_workspace()?;
        self.update_index()?;

        Ok(())
    }

    fn plan_changes(&mut self) -> anyhow::Result<()> {
        let changes: Vec<(PathBuf, TreeChangeType)> = self
            .tree_diff
            .changes()
            .iter()
            .map(|(path, change)| (path.clone(), change.clone()))
            .collect();

        for (path, change) in &changes {
            self.check_for_conflict(path, change)?;
            self.record_change(path, change)?;
        }

        let errors = self.collect_errors();

        if !errors.is_empty() {
            let errors = errors
                .iter()
                .map(|e| format!("error: {}", e))
                .collect::<Vec<_>>()
                .join("\n\n");
            anyhow::bail!("\n{}\n\nAborting", errors);
        }

        Ok(())
    }

    fn collect_errors(&self) -> Vec<String> {
        self.conflicts
            .iter()
            .filter_map(|(conflict_type, paths)| {
                if paths.is_empty() {
                    None
                } else {
                    let paths = paths
                        .iter()
                        .map(|p| format!("\t{}", p.display()))
                        .collect::<Vec<String>>();

                    let ConflictMessage { header, footer } = conflict_type.into();
                    let message = format!("{}\n{}\n{}", header, paths.join("\n"), footer);
                    Some(message)
                }
            })
            .collect::<Vec<_>>()
    }

    fn check_for_conflict(&mut self, path: &Path, change: &TreeChangeType) -> anyhow::Result<()> {
        let entry = self.index.entry_by_path(path);

        let (old_entry, new_entry) = match change {
            TreeChangeType::Added(new_entry) => (None, Some(new_entry)),
            TreeChangeType::Deleted(old_entry) => (Some(old_entry), None),
            TreeChangeType::Modified { old, new } => (Some(old), Some(new)),
        };

        if self.index_differs_from_trees(entry, old_entry, new_entry)? {
            self.conflicts
                .entry(ConflictType::StaleFile)
                .or_default()
                .push(path.into());

            return Ok(());
        }

        let stat = self.repository.workspace().stat_file(path).ok();
        let stat = stat.as_ref();
        let conflict_type = ConflictType::get_conflict_type(stat, entry, new_entry);

        match stat {
            Some(stat) if stat.mode.is_tree() => {
                if self.inspector.is_indirectly_tracked(path, self.index)? {
                    self.conflicts
                        .entry(conflict_type)
                        .or_default()
                        .push(path.into());
                }
            }
            Some(_) => {
                if self.inspector.check_index_against_workspace(entry, stat)?
                    != WorkspaceChangeType::None
                {
                    self.conflicts
                        .entry(conflict_type)
                        .or_default()
                        .push(path.into());
                }
            }
            None => {
                let parent = self.untracked_parent(path);
                if let Some(parent) = parent {
                    self.conflicts
                        .entry(conflict_type)
                        .or_default()
                        .push(if entry.is_some() {
                            path.into()
                        } else {
                            parent.into()
                        });
                }
            }
        }

        Ok(())
    }

    fn untracked_parent<'p>(&self, path: &'p Path) -> Option<&'p Path> {
        path.parent()?.ancestors().find(|parent| {
            if parent.as_os_str() == "." {
                return false;
            }

            match self.repository.workspace().stat_file(parent) {
                Ok(parent_stat) if parent_stat.mode.is_tree() => false,
                Ok(_) => self
                    .inspector
                    .is_indirectly_tracked(parent, self.index)
                    .unwrap_or_default(),
                _ => false,
            }
        })
    }

    fn index_differs_from_trees(
        &self,
        index_entry: Option<&IndexEntry>,
        old_entry: Option<&DatabaseEntry>,
        new_entry: Option<&DatabaseEntry>,
    ) -> anyhow::Result<bool> {
        Ok(self
            .inspector
            .check_index_against_head_tree(index_entry, old_entry)
            != IndexChangeType::None
            && self
                .inspector
                .check_index_against_head_tree(index_entry, new_entry)
                != IndexChangeType::None)
    }

    fn record_change(&mut self, path: &Path, change: &TreeChangeType) -> anyhow::Result<()> {
        match change {
            TreeChangeType::Added(new_entry) => {
                path.ancestors().for_each(|ancestor| {
                    if ancestor.as_os_str().is_empty() {
                        return;
                    }
                    self.mkdirs.insert(ancestor.to_path_buf());
                });

                self.actions
                    .entry(ActionType::Add)
                    .or_default()
                    .push((path.into(), Some(new_entry.clone())));
            }
            TreeChangeType::Deleted(_old_entry) => {
                path.ancestors().for_each(|ancestor| {
                    if ancestor.as_os_str().is_empty() || ancestor.is_file() {
                        return;
                    }
                    self.rmdirs.insert(ancestor.to_path_buf());
                });

                self.actions
                    .entry(ActionType::Delete)
                    .or_default()
                    .push((path.into(), None));
            }
            TreeChangeType::Modified {
                old: _old_entry,
                new: new_entry,
            } => {
                path.ancestors().for_each(|ancestor| {
                    if ancestor.as_os_str().is_empty() || ancestor.is_file() {
                        return;
                    }
                    self.mkdirs.insert(ancestor.to_path_buf());
                });

                self.actions
                    .entry(ActionType::Modify)
                    .or_default()
                    .push((path.into(), Some(new_entry.clone())));
            }
        }

        Ok(())
    }

    fn update_workspace(&self) -> anyhow::Result<()> {
        self.repository.workspace().apply_migration(self)?;

        Ok(())
    }

    fn update_index(&mut self) -> anyhow::Result<()> {
        [ActionType::Delete, ActionType::Add, ActionType::Modify]
            .iter()
            .map(|action_type| {
                self.actions
                    .get(action_type)
                    .ok_or_else(|| anyhow::anyhow!("Invalid action type"))?
                    .iter()
                    .map(|(file_path, entry)| match action_type {
                        ActionType::Delete => self.index.remove(file_path.to_path_buf()),
                        ActionType::Add | ActionType::Modify => {
                            if let Some(entry) = entry {
                                let stat = self.repository.workspace().stat_file(file_path)?;
                                self.index.add(IndexEntry::new(
                                    file_path.to_path_buf(),
                                    entry.oid.clone(),
                                    stat,
                                ))
                            } else {
                                Err(anyhow::anyhow!(
                                    "Entry must be provided for Add and Modify actions"
                                ))
                            }
                        }
                    })
                    .collect::<anyhow::Result<Vec<_>>>()?;

                Ok(())
            })
            .collect::<anyhow::Result<Vec<_>>>()?;

        Ok(())
    }

    pub fn load_blob_data(&self, object_id: &ObjectId) -> anyhow::Result<String> {
        let blob = self
            .repository
            .database()
            .parse_object_as_blob(object_id)?
            .with_context(|| format!("Failed to parse blob object {}", object_id))?;

        let content = blob.content();
        Ok(content.to_string())
    }
}
