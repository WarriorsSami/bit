use crate::domain::objects::index_entry::EntryMetadata;
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

    pub fn list_files(&self, root_file_path: Option<PathBuf>) -> Vec<PathBuf> {
        let root_file_path = root_file_path.unwrap_or_else(|| self.path.clone().into());

        if root_file_path.is_dir() {
            WalkDir::new(&root_file_path)
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
                .collect::<Vec<_>>()
        } else {
            vec![root_file_path.file_name().map(PathBuf::from).unwrap_or_default()]
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
