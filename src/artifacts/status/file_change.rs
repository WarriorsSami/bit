use crate::artifacts::index::index_entry::MergeStage;
use colored::Colorize;
use std::collections::BTreeSet;

const LABEL_WIDTH: usize = 8;

/// The type of conflict represented by non-zero stage combinations in the index.
///
/// Derived from which of stages 1 (base), 2 (ours), 3 (theirs) are present.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConflictType {
    /// Stages 1+2+3: both sides modified the file
    BothModified,
    /// Stages 1+2, no 3: their side deleted, our side modified
    DeletedByThem,
    /// Stages 1+3, no 2: our side deleted, their side modified
    DeletedByUs,
    /// Stages 2+3, no 1: both sides independently added the file
    BothAdded,
    /// Stage 2 only: we added a file that does not exist on their side
    AddedByUs,
    /// Stage 3 only: they added a file that does not exist on our side
    AddedByThem,
}

impl ConflictType {
    /// Derive the conflict type from the set of stages present for a path.
    pub fn from_stages(stages: &BTreeSet<MergeStage>) -> Self {
        let has_base = stages.contains(&MergeStage::Base);
        let has_ours = stages.contains(&MergeStage::Ours);
        let has_theirs = stages.contains(&MergeStage::Theirs);
        match (has_base, has_ours, has_theirs) {
            (true, true, true) => ConflictType::BothModified,
            (true, true, false) => ConflictType::DeletedByThem,
            (true, false, true) => ConflictType::DeletedByUs,
            (false, true, true) => ConflictType::BothAdded,
            (false, true, false) => ConflictType::AddedByUs,
            (false, false, true) => ConflictType::AddedByThem,
            _ => ConflictType::BothModified,
        }
    }

    /// Two-letter porcelain status code for machine-readable output.
    pub fn porcelain_code(&self) -> &'static str {
        match self {
            ConflictType::BothModified => "UU",
            ConflictType::DeletedByThem => "UD",
            ConflictType::DeletedByUs => "DU",
            ConflictType::BothAdded => "AA",
            ConflictType::AddedByUs => "AU",
            ConflictType::AddedByThem => "UA",
        }
    }

    /// Human-readable label for the long-format "Unmerged paths" section.
    pub fn long_label(&self) -> &'static str {
        match self {
            ConflictType::BothModified => "both modified:   ",
            ConflictType::DeletedByThem => "deleted by them: ",
            ConflictType::DeletedByUs => "deleted by us:   ",
            ConflictType::BothAdded => "both added:      ",
            ConflictType::AddedByUs => "added by us:     ",
            ConflictType::AddedByThem => "added by them:   ",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Default)]
pub enum WorkspaceChangeType {
    #[default]
    None,
    Untracked,
    Modified,
    Deleted,
}

impl From<&WorkspaceChangeType> for &str {
    fn from(change: &WorkspaceChangeType) -> Self {
        match change {
            WorkspaceChangeType::None => " ",
            WorkspaceChangeType::Untracked => "??",
            WorkspaceChangeType::Modified => "M",
            WorkspaceChangeType::Deleted => "D",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Default)]
pub enum IndexChangeType {
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
pub enum FileChangeType {
    Workspace(WorkspaceChangeType),
    Index(IndexChangeType),
}

impl From<&FileChangeType> for &str {
    fn from(change: &FileChangeType) -> Self {
        match change {
            FileChangeType::Workspace(workspace_change) => match workspace_change {
                WorkspaceChangeType::None => "",
                WorkspaceChangeType::Untracked => "",
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
            FileChangeType::Workspace(workspace_change) => match workspace_change {
                WorkspaceChangeType::None => "".normal(),
                WorkspaceChangeType::Untracked => "".normal(),
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
pub struct FileChange {
    pub(crate) workspace_change: WorkspaceChangeType,
    pub(crate) index_change: IndexChangeType,
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
