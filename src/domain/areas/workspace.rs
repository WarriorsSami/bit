use crate::domain::objects::entry::EntryMode;
use is_executable::IsExecutable;
use std::path::{Path, PathBuf};

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
    
    pub fn list_files(&self, dir: &Path) -> anyhow::Result<Vec<PathBuf>> {
        let file_names = std::fs::read_dir(dir)?
            .flatten()
            .filter_map(|entry| {
                let path = entry.path();
                let file_name = path.file_name()?.to_string_lossy().to_string();

                if IGNORED_PATHS.contains(&file_name.as_str()) {
                    return None;
                }
                
                if path.is_dir() {
                    let nested_files = self.list_files(&path);
                    Some(nested_files)
                } else {
                    // return relative file path from workspace root
                    let path = path.strip_prefix(&self.path).ok()?.to_path_buf();
                    Some(Ok(vec![path]))
                }
            })
            .flatten()
            .flatten()
            .collect();

        Ok(file_names)
    }

    pub fn read_file(&self, file_path: &Path) -> anyhow::Result<String> {
        let file_path = self.path.join(file_path);

        let content = std::fs::read_to_string(file_path)?;

        Ok(content)
    }

    pub fn stat_file(&self, file_path: &Path) -> anyhow::Result<EntryMode> {
        let file_path = self.path.join(file_path);

        let metadata = std::fs::metadata(&file_path)?;

        if metadata.is_dir() {
            Ok(EntryMode::Directory)
        } else {
            match file_path.is_executable() {
                true => Ok(EntryMode::Executable),
                false => Ok(EntryMode::Regular),
            }
        }
    }
}
