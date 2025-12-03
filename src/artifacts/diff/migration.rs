use crate::areas::index::Index;
use crate::areas::repository::Repository;
use crate::artifacts::database::database_entry::DatabaseEntry;
use crate::artifacts::diff::tree_diff::{TreeChangeType, TreeDiff};
use crate::artifacts::index::index_entry::IndexEntry;
use crate::artifacts::objects::object_id::ObjectId;
use anyhow::Context;
use std::collections::{BTreeSet, HashMap};
use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ActionType {
    Add,
    Delete,
    Modify,
}

pub type ActionsSet = HashMap<ActionType, Vec<(PathBuf, Option<DatabaseEntry>)>>;

pub struct Migration<'r> {
    repository: &'r Repository,
    tree_diff: TreeDiff<'r>,
    index: &'r mut Index,
    actions: ActionsSet,
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

        Self {
            repository,
            index,
            tree_diff,
            actions,
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
        // Split borrows: get immutable reference to tree_diff, then process entries
        // by splitting self into its components
        let Migration {
            tree_diff,
            actions,
            mkdirs,
            rmdirs,
            ..
        } = self;

        for (path, change) in tree_diff.changes().iter() {
            match change {
                TreeChangeType::Added(new_entry) => {
                    path.ancestors().for_each(|ancestor| {
                        if ancestor.as_os_str().is_empty() {
                            return;
                        }
                        mkdirs.insert(ancestor.to_path_buf());
                    });

                    actions
                        .entry(ActionType::Add)
                        .or_insert_with(Vec::new)
                        .push((path.clone(), Some(new_entry.clone())));
                }
                TreeChangeType::Deleted(_old_entry) => {
                    path.ancestors().for_each(|ancestor| {
                        if ancestor.as_os_str().is_empty() || ancestor.is_file() {
                            return;
                        }
                        rmdirs.insert(ancestor.to_path_buf());
                    });

                    actions
                        .entry(ActionType::Delete)
                        .or_insert_with(Vec::new)
                        .push((path.clone(), None));
                }
                TreeChangeType::Modified {
                    old: _old_entry,
                    new: new_entry,
                } => {
                    path.ancestors().for_each(|ancestor| {
                        if ancestor.as_os_str().is_empty() || ancestor.is_file() {
                            return;
                        }
                        mkdirs.insert(ancestor.to_path_buf());
                    });

                    actions
                        .entry(ActionType::Modify)
                        .or_insert_with(Vec::new)
                        .push((path.clone(), Some(new_entry.clone())));
                }
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
