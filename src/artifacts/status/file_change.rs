use colored::Colorize;

const LABEL_WIDTH: usize = 8;

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
