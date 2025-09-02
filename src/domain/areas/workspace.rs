use crate::domain::objects::index_entry::EntryMetadata;
use anyhow::anyhow;
use std::fs::metadata;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

const IGNORED_PATHS: [&str; 3] = [".git", ".", ".."];

pub struct Workspace {
    path: Box<Path>,
}

impl Workspace {
    pub fn new(path: Box<Path>) -> Self {
        Workspace { path }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn list_dir(&self, dir_path: Option<&Path>) -> anyhow::Result<Vec<PathBuf>> {
        let dir_path = match dir_path {
            Some(p) => std::fs::canonicalize(p)?,
            None => self.path.clone().into(),
        };

        // Check if the dir_path exists
        if !dir_path.exists() {
            return Err(anyhow!("The specified path does not exist: {:?}", dir_path));
        }

        if dir_path.is_dir() {
            Ok(std::fs::read_dir(&dir_path)?
                .filter_map(|entry| entry.ok())
                .filter_map(|entry| self.check_if_not_ignored_path(&entry.path()))
                .collect::<Vec<_>>())
        } else {
            Err(anyhow!(
                "The specified path is not a directory: {:?}",
                dir_path
            ))
        }
    }

    // TODO: refactor to use iterator
    pub fn list_files(&self, root_file_path: Option<PathBuf>) -> anyhow::Result<Vec<PathBuf>> {
        let root_file_path = match root_file_path {
            Some(p) => std::fs::canonicalize(p)?,
            None => self.path.clone().into(),
        };

        // Check if the root_file_path exists
        if !root_file_path.exists() {
            return Err(anyhow!(
                "The specified path does not exist: {:?}",
                root_file_path
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

    pub fn read_file(&self, file_path: &Path) -> anyhow::Result<String> {
        let file_path = self.path.join(file_path);

        let content = std::fs::read_to_string(file_path)?;

        Ok(content)
    }

    pub fn stat_file(&self, file_path: &Path) -> anyhow::Result<EntryMetadata> {
        let metadata = metadata(self.path.join(file_path))?;

        (file_path, metadata).try_into()
    }
}
