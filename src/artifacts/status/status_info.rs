use crate::areas::index::Index;
use crate::areas::repository::Repository;
use crate::artifacts::database::database_entry::DatabaseEntry;
use crate::artifacts::index::index_entry::{EntryMetadata, IndexEntry};
use crate::artifacts::status::file_change::{
    FileChange, FileChangeType, IndexChangeType, WorkspaceChangeType,
};
use crate::artifacts::status::inspector::Inspector;
use derive_new::new;
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

// TODO: use the file change types separately for each area change (untracked, workspace, index)
pub type FileStatSet = BTreeMap<PathBuf, EntryMetadata>;
pub type ChangeSet = BTreeMap<PathBuf, FileChangeType>;
pub type FileSet = BTreeSet<PathBuf>;
pub type HeadTree = BTreeMap<PathBuf, DatabaseEntry>;

#[derive(Debug, Clone)]
pub struct StatusInfo {
    pub(crate) file_stats: FileStatSet,
    pub(crate) untracked_files: FileSet,
    pub(crate) changed_files: BTreeMap<PathBuf, FileChange>,
    pub(crate) untracked_changeset: ChangeSet,
    pub(crate) workspace_changeset: ChangeSet,
    pub(crate) index_changeset: ChangeSet,
    pub(crate) head_tree: HeadTree,
}

#[derive(new)]
pub struct Status<'r> {
    repository: &'r Repository,
}

impl<'r> Status<'r> {
    pub async fn initialize(&self, index: &mut Index) -> anyhow::Result<StatusInfo> {
        let mut file_stats = BTreeMap::<PathBuf, EntryMetadata>::new();
        let mut untracked_files = BTreeSet::<PathBuf>::new();

        let inspector = Inspector::new(self.repository);

        self.scan_workspace(
            None,
            &mut untracked_files,
            &mut file_stats,
            index,
            &inspector,
        )
        .await?;
        let head_tree = self.load_head_tree().await?;
        let mut changed_files =
            self.check_index_entries(&file_stats, &head_tree, index, &inspector)?;
        self.collect_deleted_head_files(&head_tree, index, &mut changed_files);

        let untracked_changeset = untracked_files
            .iter()
            .map(|file| (file.clone(), FileChangeType::Untracked))
            .collect::<BTreeMap<_, _>>();
        let workspace_changeset = changed_files
            .iter()
            .filter(|(_, change)| change.workspace_change != WorkspaceChangeType::None)
            .map(|(file, change)| {
                (
                    file.clone(),
                    FileChangeType::Workspace(change.workspace_change.clone()),
                )
            })
            .collect::<BTreeMap<_, _>>();
        let index_changeset = changed_files
            .iter()
            .filter(|(_, change)| change.index_change != IndexChangeType::None)
            .map(|(file, change)| {
                (
                    file.clone(),
                    FileChangeType::Index(change.index_change.clone()),
                )
            })
            .collect::<BTreeMap<_, _>>();

        Ok(StatusInfo {
            file_stats,
            untracked_files,
            changed_files,
            untracked_changeset,
            workspace_changeset,
            index_changeset,
            head_tree,
        })
    }

    async fn scan_workspace(
        &self,
        prefix_path: Option<&Path>,
        untracked_files: &mut BTreeSet<PathBuf>,
        file_stats: &mut BTreeMap<PathBuf, EntryMetadata>,
        index: &Index,
        inspector: &Inspector<'_>,
    ) -> anyhow::Result<()> {
        let files = self.repository.workspace().list_dir(prefix_path)?;

        for path in files.iter() {
            if index.is_directly_tracked(path) {
                if path.is_dir() {
                    Box::pin(self.scan_workspace(
                        Some(path),
                        untracked_files,
                        file_stats,
                        index,
                        inspector,
                    ))
                    .await?;
                } else {
                    let stat = self.repository.workspace().stat_file(path)?;
                    file_stats.insert(path.clone(), stat);
                }
            } else if !inspector.is_indirectly_tracked(path, index)? {
                // add the file separator if it's a directory
                let path = if path.is_dir() {
                    let mut p = path.clone();
                    p.push("");
                    p
                } else {
                    path.clone()
                };
                untracked_files.insert(path);
            }
        }

        Ok(())
    }

    async fn load_head_tree(&self) -> anyhow::Result<BTreeMap<PathBuf, DatabaseEntry>> {
        let mut head_tree = BTreeMap::<PathBuf, DatabaseEntry>::new();

        if let Some(head_ref) = self.repository.refs().read_head()? {
            let commit = self
                .repository
                .database()
                .parse_object_as_commit(&head_ref)?;

            if let Some(commit) = commit {
                self.repository
                    .parse_tree(commit.tree_oid(), None, &mut head_tree, false)
                    .await?;
            }
        }

        Ok(head_tree)
    }

    fn check_index_entries(
        &self,
        file_stats: &BTreeMap<PathBuf, EntryMetadata>,
        head_tree: &BTreeMap<PathBuf, DatabaseEntry>,
        index: &mut Index,
        inspector: &Inspector<'_>,
    ) -> anyhow::Result<BTreeMap<PathBuf, FileChange>> {
        let mut changed_files = BTreeMap::<PathBuf, FileChange>::new();
        let index_entries = index.entries().map(Clone::clone).collect::<Vec<_>>();

        for entry in index_entries {
            self.check_index_entry_against_workspace(
                &entry,
                file_stats,
                index,
                inspector,
                &mut changed_files,
            )?;
            self.check_index_entry_against_head_tree(
                &entry,
                head_tree,
                inspector,
                &mut changed_files,
            )?;
        }

        Ok(changed_files)
    }

    fn check_index_entry_against_workspace(
        &self,
        index_entry: &IndexEntry,
        file_stats: &BTreeMap<PathBuf, EntryMetadata>,
        index: &mut Index,
        inspector: &Inspector<'_>,
        changed_files: &mut BTreeMap<PathBuf, FileChange>,
    ) -> anyhow::Result<()> {
        let stat = file_stats.get(&index_entry.name);
        let status = inspector.check_index_against_workspace(index_entry, stat)?;

        if status != WorkspaceChangeType::None {
            self.record_workspace_change(index_entry.name.clone(), status, changed_files);
        } else if let Some(stat) = stat {
            index.update_entry_stat(index_entry, stat.clone());
        }

        Ok(())
    }

    fn record_workspace_change(
        &self,
        entry_path: PathBuf,
        change: WorkspaceChangeType,
        changed_files: &mut BTreeMap<PathBuf, FileChange>,
    ) {
        changed_files
            .entry(entry_path)
            .or_default()
            .workspace_change = change;
    }

    fn check_index_entry_against_head_tree(
        &self,
        index_entry: &IndexEntry,
        head_tree: &BTreeMap<PathBuf, DatabaseEntry>,
        inspector: &Inspector<'_>,
        changed_files: &mut BTreeMap<PathBuf, FileChange>,
    ) -> anyhow::Result<()> {
        let head_entry = head_tree.get(&index_entry.name);
        let status = inspector.check_index_against_head_tree(index_entry, head_entry)?;

        if status != IndexChangeType::None {
            self.record_index_change(index_entry.name.clone(), status, changed_files);
        }

        Ok(())
    }

    fn record_index_change(
        &self,
        entry_path: PathBuf,
        change: IndexChangeType,
        changed_files: &mut BTreeMap<PathBuf, FileChange>,
    ) {
        changed_files.entry(entry_path).or_default().index_change = change;
    }

    fn collect_deleted_head_files(
        &self,
        head_tree: &BTreeMap<PathBuf, DatabaseEntry>,
        index: &mut Index,
        changed_files: &mut BTreeMap<PathBuf, FileChange>,
    ) {
        head_tree.iter().for_each(|(path, _)| {
            if !index.is_directly_tracked(path) {
                changed_files.entry(path.clone()).or_default().index_change =
                    IndexChangeType::Deleted;
            }
        });
    }
}
