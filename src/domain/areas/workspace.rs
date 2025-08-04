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

    pub fn list_files(&self) -> Vec<PathBuf> {
        WalkDir::new(&self.path)
            .into_iter()
            .filter_map(|entry| {
                let entry = entry.ok()?;
                let path = entry.path();

                let file_name = path.file_name()?.to_string_lossy().to_string();

                if path.is_file() && !IGNORED_PATHS.contains(&file_name.as_str()) {
                    Some(path.strip_prefix(&self.path).ok()?.to_path_buf())
                } else {
                    None
                }
            })
            .collect::<Vec<_>>()
    }

    pub fn read_file(&self, file_path: &Path) -> anyhow::Result<String> {
        let file_path = self.path.join(file_path);

        let content = std::fs::read_to_string(file_path)?;

        Ok(content)
    }

    pub fn stat_file(&self, file_path: &Path) -> anyhow::Result<EntryMetadata> {
        let file_path = self.path.join(file_path);
        let metadata = metadata(&file_path)?;

        (&file_path, metadata).try_into()
    }
}
