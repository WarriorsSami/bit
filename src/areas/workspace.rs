//! Working directory operations
//!
//! The workspace module provides an abstraction over file system operations
//! in the working directory (the user's project files).
//!
//! ## Responsibilities
//!
//! - Reading and writing files
//! - Listing directories recursively
//! - Tracking file metadata (mode, timestamps)
//! - Filtering out ignored files and directories (.git, etc.)
//! - Applying checkout migrations (creating, updating, deleting files)

use crate::artifacts::checkout::migration::{ActionType, Migration};
use crate::artifacts::index::index_entry::{EntryMetadata, IndexEntryError};
use crate::artifacts::objects::blob::Blob;
use std::io::Write;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

#[derive(Debug, thiserror::Error)]
pub enum WorkspaceError {
    #[error(transparent)]
    IndexEntry(#[from] IndexEntryError),
    #[error("path does not exist: {0}")]
    PathNotFound(String),
    #[error("path is not a directory: {0}")]
    NotADirectory(String),
    #[error("invalid action type")]
    InvalidActionType,
    #[error("invalid action and entry combination")]
    InvalidActionEntry,
    #[error("{operation} failed for {path}")]
    FileOperation {
        operation: &'static str,
        path: String,
        #[source]
        source: std::io::Error,
    },
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Internal(#[from] anyhow::Error),
}

/// Paths that should always be ignored when scanning the workspace
const IGNORED_PATHS: [&str; 3] = [".git", ".", ".."];

/// Working directory abstraction
///
/// Provides file system operations for the Git working tree.
/// All file paths are resolved relative to the workspace root.
#[derive(Debug)]
pub struct Workspace {
    /// Root path of the working directory
    path: Box<Path>,
}

impl Workspace {
    pub fn new(path: Box<Path>) -> Self {
        Workspace { path }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Parse a file into a Blob object
    ///
    /// Reads the file content and creates a Blob with default mode.
    ///
    /// # Arguments
    ///
    /// * `path` - Path to the file relative to workspace root
    pub fn parse_blob(&self, path: &Path) -> Result<Blob, WorkspaceError> {
        let data = self.read_file(path)?;
        Ok(Blob::new(data, Default::default()))
    }

    /// List immediate children of a directory
    ///
    /// Returns only direct children, not recursive. Filters out ignored paths.
    ///
    /// # Arguments
    ///
    /// * `dir_path` - Directory to list (None for workspace root)
    ///
    /// # Returns
    ///
    /// Vector of paths to children, relative to workspace root
    pub fn list_dir(&self, dir_path: Option<&Path>) -> Result<Vec<PathBuf>, WorkspaceError> {
        let dir_path = match dir_path {
            Some(p) => std::fs::canonicalize(p)?,
            None => self.path.clone().into(),
        };

        if !dir_path.exists() {
            return Err(WorkspaceError::PathNotFound(dir_path.display().to_string()));
        }

        if dir_path.is_dir() {
            Ok(std::fs::read_dir(&dir_path)?
                .filter_map(|entry| entry.ok())
                .filter_map(|entry| self.check_if_not_ignored_path(&entry.path()))
                .collect::<Vec<_>>())
        } else {
            Err(WorkspaceError::NotADirectory(
                dir_path.display().to_string(),
            ))
        }
    }

    /// List all files recursively
    ///
    /// Walks the directory tree and returns all non-ignored files.
    ///
    /// # Arguments
    ///
    /// * `root_file_path` - Starting path (None for workspace root)
    ///
    /// # Returns
    ///
    /// Vector of file paths relative to workspace root
    // TODO: refactor to use iterator
    pub fn list_files(
        &self,
        root_file_path: Option<PathBuf>,
    ) -> Result<Vec<PathBuf>, WorkspaceError> {
        let root_file_path = match root_file_path {
            Some(p) => std::fs::canonicalize(p)?,
            None => self.path.clone().into(),
        };

        if !root_file_path.exists() {
            return Err(WorkspaceError::PathNotFound(
                root_file_path.display().to_string(),
            ));
        }

        if root_file_path.is_dir() {
            Ok(WalkDir::new(&root_file_path)
                .into_iter()
                .filter_map(|entry| entry.ok())
                .filter_map(|entry| self.check_if_not_ignored_file_path(entry.path()))
                .collect::<Vec<_>>())
        } else {
            Ok(vec![
                root_file_path
                    .strip_prefix(self.path.as_ref())
                    .map(PathBuf::from)
                    .unwrap_or_default(),
            ])
        }
    }

    /// Check if a path should be ignored
    ///
    /// Checks against IGNORED_PATHS (.git, ., ..)
    fn is_ignored(path: &Path) -> bool {
        // Check if any component of the path is in IGNORED_PATHS
        path.components().any(|component| {
            if let std::path::Component::Normal(name) = component {
                let name_str = name.to_string_lossy();
                IGNORED_PATHS.contains(&name_str.as_ref())
            } else {
                false
            }
        })
    }

    fn check_if_not_ignored_path(&self, path: &Path) -> Option<PathBuf> {
        if !Self::is_ignored(path) {
            Some(path.strip_prefix(self.path.as_ref()).ok()?.to_path_buf())
        } else {
            None
        }
    }

    fn check_if_not_ignored_file_path(&self, path: &Path) -> Option<PathBuf> {
        if path.is_file() && !Self::is_ignored(path) {
            Some(path.strip_prefix(self.path.as_ref()).ok()?.to_path_buf())
        } else {
            None
        }
    }

    pub fn read_file(&self, file_path: &Path) -> Result<String, WorkspaceError> {
        let file_path = self.path.join(file_path);

        let content = std::fs::read_to_string(file_path)?;

        Ok(content)
    }

    pub fn write_file(&self, file_path: &Path, content: &[u8]) -> Result<(), WorkspaceError> {
        let full_path = self.path.join(file_path);
        std::fs::write(full_path, content)?;
        Ok(())
    }

    pub fn rename_file(&self, from: &Path, to: &Path) -> Result<(), WorkspaceError> {
        std::fs::rename(self.path.join(from), self.path.join(to))?;
        Ok(())
    }

    pub fn stat_file(&self, file_path: &Path) -> Result<EntryMetadata, WorkspaceError> {
        let metadata = std::fs::metadata(self.path.join(file_path))?;

        Ok((file_path, metadata).try_into()?)
    }

    // The order of applying migrations is important:
    // For deletions, we first delete files and then remove directories in reverse order.
    // For additions, we first create directories and then add/update files.
    pub fn apply_migration(&self, migration: &Migration) -> Result<(), WorkspaceError> {
        self.apply_migration_action_set(migration, ActionType::Delete)?;
        migration
            .rmdirs()
            .iter()
            .rev()
            .map(|dir_path| self.remove_directory(dir_path))
            .collect::<Result<Vec<()>, _>>()?;

        migration
            .mkdirs()
            .iter()
            .map(|dir_path| self.make_directory(dir_path))
            .collect::<Result<Vec<()>, _>>()?;
        self.apply_migration_action_set(migration, ActionType::Modify)?;
        self.apply_migration_action_set(migration, ActionType::Add)?;

        Ok(())
    }

    fn apply_migration_action_set(
        &self,
        migration: &Migration,
        action: ActionType,
    ) -> Result<(), WorkspaceError> {
        migration
            .actions()
            .get(&action)
            .ok_or(WorkspaceError::InvalidActionType)?
            .iter()
            .map(|(file_path, entry)| {
                let path = self.path.join(file_path);

                if path.exists() {
                    let metadata =
                        std::fs::metadata(&path).map_err(|e| WorkspaceError::FileOperation {
                            operation: "stat",
                            path: file_path.display().to_string(),
                            source: e,
                        })?;

                    if metadata.is_dir() {
                        std::fs::remove_dir_all(&path).map_err(|e| {
                            WorkspaceError::FileOperation {
                                operation: "remove directory",
                                path: file_path.display().to_string(),
                                source: e,
                            }
                        })?;
                    }

                    if metadata.is_file() {
                        std::fs::remove_file(&path).map_err(|e| WorkspaceError::FileOperation {
                            operation: "remove file",
                            path: file_path.display().to_string(),
                            source: e,
                        })?;
                    }
                }

                match (&action, entry) {
                    (ActionType::Delete, None) => Ok(()),
                    (ActionType::Add | ActionType::Modify, Some(entry)) => {
                        let data = migration.load_blob_data(&entry.oid)?;

                        let mut file = std::fs::OpenOptions::new()
                            .write(true)
                            .create(true)
                            .truncate(true)
                            .open(&path)
                            .map_err(|e| WorkspaceError::FileOperation {
                                operation: "open",
                                path: file_path.display().to_string(),
                                source: e,
                            })?;

                        file.write_all(data.as_bytes()).map_err(|e| {
                            WorkspaceError::FileOperation {
                                operation: "write",
                                path: file_path.display().to_string(),
                                source: e,
                            }
                        })?;

                        #[cfg(unix)]
                        {
                            use std::os::unix::fs::PermissionsExt;
                            let permissions = std::fs::Permissions::from_mode(entry.mode.as_u32());
                            std::fs::set_permissions(self.path.join(file_path), permissions)
                                .map_err(|e| WorkspaceError::FileOperation {
                                    operation: "set permissions",
                                    path: file_path.display().to_string(),
                                    source: e,
                                })?;
                        }

                        Ok(())
                    }
                    _ => Err(WorkspaceError::InvalidActionEntry),
                }
            })
            .collect::<Result<Vec<()>, _>>()?;

        Ok(())
    }

    fn remove_directory(&self, dir_path: &Path) -> Result<(), WorkspaceError> {
        let dir_path = self.path.join(dir_path);

        std::fs::remove_dir_all(dir_path)?;

        Ok(())
    }

    fn make_directory(&self, dir_path: &Path) -> Result<(), WorkspaceError> {
        let dir_path = self.path.join(dir_path);

        if !dir_path.exists() {
            std::fs::create_dir(&dir_path)?;
            return Ok(());
        }

        let metadata = std::fs::metadata(&dir_path)?;
        // delete existing file if it's a file
        if metadata.is_file() {
            std::fs::remove_file(&dir_path)?;
        }

        if !metadata.is_dir() {
            std::fs::create_dir(dir_path)?;
        }

        Ok(())
    }
}
