use std::path::Path;

const IGNORED_PATHS: [&str; 3] = [".git", ".", ".."];

pub struct Workspace {
    path: Box<Path>,
}

impl Workspace {
    pub fn new(path: Box<Path>) -> Self { 
        Workspace { path }
    }

    pub fn list_files(&self) -> anyhow::Result<Vec<String>> {
        // read only the first level of the directory (for now)
        let files = std::fs::read_dir(&self.path)?
            .flatten()
            .filter_map(|entry| {
                let path = entry.path();
                let file_name = path.file_name()?.to_string_lossy().to_string();

                if IGNORED_PATHS.contains(&file_name.as_str()) {
                    return None;
                }

                Some(file_name)
            })
            .collect();

        Ok(files)
    }

    pub fn read_file(&self, file_name: &str) -> anyhow::Result<String> {
        let file_path = self.path.join(file_name);

        let content = std::fs::read_to_string(file_path)?;

        Ok(content)
    }
}
