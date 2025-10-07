use crate::domain::areas::index::Index;
use crate::domain::areas::repository::Repository;
use crate::domain::objects::blob::Blob;
use crate::domain::objects::database_entry::DatabaseEntry;
use crate::domain::objects::index_entry::{EntryMetadata, IndexEntry};
use crate::domain::objects::object::Object;
use crate::domain::objects::object_id::ObjectId;
use colored::*;
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

const LABEL_WIDTH: usize = 8;

type ChangeSet = BTreeMap<PathBuf, FileChangeType>;

#[derive(Debug, Clone)]
struct StatusInfo {
    untracked_changeset: ChangeSet,
    workspace_changeset: ChangeSet,
    index_changeset: ChangeSet,
}

// Terminology:
// - untracked files: files that are not tracked by the index
// - workspace modified files: files that are tracked by the index but have changes in the workspace
// - workspace deleted files: files that are tracked by the index but have been deleted from the workspace
// - index added files: files that are in the index but not in the HEAD commit
// - index modified files: files that are in the index and in the HEAD commit but have different content or mode
// - index deleted files: files that are in the HEAD commit but not in the index
impl Repository {
    // TODO: define a data structure to return the status information more easily and cache it if needed inside the Repository struct
    pub async fn status(&mut self, porcelain: bool) -> anyhow::Result<()> {
        let index = self.index();
        let mut index = index.lock().await;

        index.rehydrate()?;

        let mut file_stats = BTreeMap::<PathBuf, EntryMetadata>::new();
        let mut untracked_files = BTreeSet::<PathBuf>::new();

        self.scan_workspace(None, &mut untracked_files, &mut file_stats, &index)
            .await?;
        let head_tree = self.load_head_tree().await?;
        let mut changed_files = self.check_index_entries(&file_stats, &head_tree, &mut index);
        self.collect_deleted_head_files(&head_tree, &mut index, &mut changed_files);

        if porcelain {
            changed_files.iter().for_each(|(file, status)| {
                writeln!(self.writer(), "{} {}", status, file.display()).unwrap();
            });

            untracked_files.iter().for_each(|file| {
                writeln!(self.writer(), "?? {}", file.display()).unwrap();
            });
        } else {
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

            Self::print_changes("Changes to be committed", &index_changeset);
            Self::print_changes("Changes not staged for commit", &workspace_changeset);
            Self::print_changes("Untracked files", &untracked_changeset);

            let status_info = StatusInfo {
                untracked_changeset,
                workspace_changeset,
                index_changeset,
            };
            Self::print_commit_status(&status_info);
        }

        Ok(())
    }

    fn print_changes(message: &str, changeset: &BTreeMap<PathBuf, FileChangeType>) {
        if !changeset.is_empty() {
            println!("{}:\n", message.bold());
            for (file, change) in changeset {
                println!("{}{}", change, file.display().to_string().cyan());
            }
            println!();
        }
    }

    fn print_commit_status(status_info: &StatusInfo) {
        if !status_info.index_changeset.is_empty() {
            return;
        }

        if !status_info.workspace_changeset.is_empty() {
            println!("{}", "no changes added to commit".yellow());
            return;
        }

        if !status_info.untracked_changeset.is_empty() {
            println!(
                "{}",
                "no changes added to commit but untracked files present".yellow()
            );
            return;
        }

        println!("{}", "nothing to commit, working tree clean".green());
    }

    async fn scan_workspace(
        &self,
        prefix_path: Option<&Path>,
        untracked_files: &mut BTreeSet<PathBuf>,
        file_stats: &mut BTreeMap<PathBuf, EntryMetadata>,
        index: &Index,
    ) -> anyhow::Result<()> {
        let files = self.workspace().list_dir(prefix_path)?;

        for path in files.iter() {
            if index.is_directly_tracked(path) {
                if path.is_dir() {
                    Box::pin(self.scan_workspace(Some(path), untracked_files, file_stats, index))
                        .await?;
                } else {
                    let stat = self.workspace().stat_file(path)?;
                    file_stats.insert(path.clone(), stat);
                }
            } else if !self.is_indirectly_tracked(path, index)? {
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

        if let Some(head_ref) = self.refs().read_head() {
            let head_oid = ObjectId::try_parse(head_ref)?;
            let commit = self.database().parse_object_as_commit(&head_oid)?;

            if let Some(commit) = commit {
                self.parse_tree(commit.tree_oid(), None, &mut head_tree, false)
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
    ) -> BTreeMap<PathBuf, FileChange> {
        let mut changed_files = BTreeMap::<PathBuf, FileChange>::new();

        self.check_index_against_workspace(file_stats, index, &mut changed_files);
        self.check_index_against_head(head_tree, index, &mut changed_files);

        changed_files
    }

    fn check_index_against_workspace(
        &self,
        file_stats: &BTreeMap<PathBuf, EntryMetadata>,
        index: &mut Index,
        changed_files: &mut BTreeMap<PathBuf, FileChange>,
    ) {
        // TODO: optimize by avoiding cloning all entries
        let index_entries = index.entries().map(Clone::clone).collect::<Vec<_>>();

        let modified_files = index_entries
            .into_iter()
            .filter_map(|entry| {
                if let Some(stat) = file_stats.get(&entry.name) {
                    Some((entry, stat))
                } else {
                    // file deleted
                    changed_files
                        .entry(entry.name.clone())
                        .or_default()
                        .workspace_change = WorkspaceChangeType::Deleted;

                    None
                }
            })
            .filter_map(|(index_entry, workspace_stat)| {
                match index_entry.stat_match(workspace_stat) {
                    true if index_entry.times_match(workspace_stat) => None,
                    true => self.is_content_changed(&index_entry).ok().map(|changed| {
                        if changed {
                            Some(index_entry.name.clone())
                        } else {
                            index.update_entry_stat(&index_entry, workspace_stat.clone());
                            None
                        }
                    })?,
                    false => Some(index_entry.name.clone()),
                }
            })
            .collect::<Vec<_>>();

        for path in modified_files {
            changed_files.entry(path).or_default().workspace_change = WorkspaceChangeType::Modified;
        }
    }

    fn check_index_against_head(
        &self,
        head_tree: &BTreeMap<PathBuf, DatabaseEntry>,
        index: &mut Index,
        changed_files: &mut BTreeMap<PathBuf, FileChange>,
    ) {
        // TODO: optimize by avoiding cloning all entries
        let index_entries = index.entries().map(Clone::clone).collect::<Vec<_>>();

        index_entries.into_iter().for_each(|entry| {
            if let Some(head_entry) = head_tree.get(&entry.name)
                && (head_entry.mode != entry.metadata.mode || head_entry.oid != entry.oid)
            {
                changed_files
                    .entry(entry.name.clone())
                    .or_default()
                    .index_change = IndexChangeType::Modified;
            } else if !head_tree.contains_key(&entry.name) {
                changed_files
                    .entry(entry.name.clone())
                    .or_default()
                    .index_change = IndexChangeType::Added;
            }
        });
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

    fn is_content_changed(&self, index_entry: &IndexEntry) -> anyhow::Result<bool> {
        let data = self.workspace().read_file(&index_entry.name)?;
        let blob = Blob::new(data, Default::default());
        let oid = blob.object_id()?;

        Ok(oid != index_entry.oid)
    }

    fn is_indirectly_tracked(&self, path: &Path, index: &Index) -> anyhow::Result<bool> {
        if path.is_file() {
            return Ok(index.is_directly_tracked(path));
        }

        let paths = self.workspace().list_dir(Some(path))?;
        let files = paths.iter().filter(|p| p.is_file());
        let dirs = paths.iter().filter(|p| p.is_dir());

        let mut paths = files.chain(dirs);

        // chain the iterators and check if any of the files or directories are indirectly tracked
        if paths.clone().count() == 0 {
            Ok(true)
        } else {
            Ok(paths.any(|p| self.is_indirectly_tracked(p, index).unwrap_or(false)))
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Default)]
enum WorkspaceChangeType {
    #[default]
    None,
    Modified,
    Deleted,
}

impl From<&WorkspaceChangeType> for &str {
    fn from(change: &WorkspaceChangeType) -> Self {
        match change {
            WorkspaceChangeType::None => " ",
            WorkspaceChangeType::Modified => "M",
            WorkspaceChangeType::Deleted => "D",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Default)]
enum IndexChangeType {
    #[default]
    None,
    Added,
    Modified,
    Deleted,
}

impl From<&IndexChangeType> for &str {
    fn from(change: &IndexChangeType) -> Self {
        match change {
            IndexChangeType::None => " ",
            IndexChangeType::Added => "A",
            IndexChangeType::Modified => "M",
            IndexChangeType::Deleted => "D",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
enum FileChangeType {
    Untracked,
    Workspace(WorkspaceChangeType),
    Index(IndexChangeType),
}

impl From<&FileChangeType> for &str {
    fn from(change: &FileChangeType) -> Self {
        match change {
            FileChangeType::Untracked => "",
            FileChangeType::Workspace(workspace_change) => match workspace_change {
                WorkspaceChangeType::None => "",
                WorkspaceChangeType::Modified => "modified:   ",
                WorkspaceChangeType::Deleted => "deleted:    ",
            },
            FileChangeType::Index(index_change) => match index_change {
                IndexChangeType::None => "",
                IndexChangeType::Added => "new file:   ",
                IndexChangeType::Modified => "modified:   ",
                IndexChangeType::Deleted => "deleted:   ",
            },
        }
    }
}

impl std::fmt::Display for FileChangeType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let colored_str = match self {
            FileChangeType::Untracked => "".normal(),
            FileChangeType::Workspace(workspace_change) => match workspace_change {
                WorkspaceChangeType::None => "".normal(),
                WorkspaceChangeType::Modified => "modified:   ".red(),
                WorkspaceChangeType::Deleted => "deleted:    ".red(),
            },
            FileChangeType::Index(index_change) => match index_change {
                IndexChangeType::None => "".normal(),
                IndexChangeType::Added => "new file:   ".green(),
                IndexChangeType::Modified => "modified:   ".green(),
                IndexChangeType::Deleted => "deleted:    ".green(),
            },
        };
        write!(f, "{:>width$}{}", "", colored_str, width = LABEL_WIDTH)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Default)]
struct FileChange {
    workspace_change: WorkspaceChangeType,
    index_change: IndexChangeType,
}

impl From<&FileChange> for String {
    fn from(change: &FileChange) -> Self {
        let index_str: &str = (&change.index_change).into();
        let workspace_str: &str = (&change.workspace_change).into();
        format!("{}{}", index_str, workspace_str)
    }
}

impl std::fmt::Display for FileChange {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let change_str: String = self.into();
        write!(f, "{}", change_str)
    }
}
