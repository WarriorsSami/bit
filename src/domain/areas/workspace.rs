use crate::domain::objects::index_entry::EntryMetadata;
use anyhow::{Context, anyhow};
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

    pub fn list_files(&self, root_file_path: Option<PathBuf>) -> anyhow::Result<Vec<PathBuf>> {
        let root_file_path = root_file_path.unwrap_or_else(|| self.path.clone().into());

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
                .filter_map(|entry| {
                    let entry = entry.ok()?;
                    let path = entry.path();

                    // Check if any component of the path is in IGNORED_PATHS
                    let is_ignored = path.components().any(|component| {
                        if let std::path::Component::Normal(name) = component {
                            let name_str = name.to_string_lossy();
                            IGNORED_PATHS.contains(&name_str.as_ref())
                        } else {
                            false
                        }
                    });

                    if path.is_file() && !is_ignored {
                        Some(path.strip_prefix(self.path.as_ref()).ok()?.to_path_buf())
                    } else {
                        None
                    }
                })
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

    pub fn read_file(&self, file_path: &Path) -> anyhow::Result<String> {
        let file_path = self.path.join(file_path);

        let content = std::fs::read_to_string(&file_path)
            .with_context(|| format!("Failed to read file: {:?}", file_path))?;

        Ok(content)
    }

    pub fn stat_file(&self, file_path: &Path) -> anyhow::Result<EntryMetadata> {
        let metadata = metadata(self.path.join(file_path))
            .with_context(|| format!("Failed to stat file: {:?}", file_path))?;

        (file_path, metadata).try_into()
    }
}
