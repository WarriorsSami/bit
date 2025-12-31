use crate::artifacts::checkout::migration::{ActionType, Migration};
use crate::artifacts::index::index_entry::EntryMetadata;
use crate::artifacts::objects::blob::Blob;
use anyhow::Context;
use std::io::Write;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

const IGNORED_PATHS: [&str; 3] = [".git", ".", ".."];

#[derive(Debug)]
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

    pub fn parse_blob(&self, path: &Path) -> anyhow::Result<Blob> {
        let data = self.read_file(path)?;
        Ok(Blob::new(data, Default::default()))
    }

    pub fn list_dir(&self, dir_path: Option<&Path>) -> anyhow::Result<Vec<PathBuf>> {
        let dir_path = match dir_path {
            Some(p) => std::fs::canonicalize(p)?,
            None => self.path.clone().into(),
        };

        // Check if the dir_path exists
        if !dir_path.exists() {
            anyhow::bail!("The specified path does not exist: {:?}", dir_path);
        }

        if dir_path.is_dir() {
            Ok(std::fs::read_dir(&dir_path)?
                .filter_map(|entry| entry.ok())
                .filter_map(|entry| self.check_if_not_ignored_path(&entry.path()))
                .collect::<Vec<_>>())
        } else {
            anyhow::bail!("The specified path is not a directory: {:?}", dir_path);
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
            anyhow::bail!("The specified path does not exist: {:?}", root_file_path);
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
        let metadata = std::fs::metadata(self.path.join(file_path))?;

        (file_path, metadata).try_into()
    }

    // The order of applying migrations is important:
    // For deletions, we first delete files and then remove directories in reverse order.
    // For additions, we first create directories and then add/update files.
    pub fn apply_migration(&self, migration: &Migration) -> anyhow::Result<()> {
        self.apply_migration_action_set(migration, ActionType::Delete)?;
        // here we remove directories in reverse order to ensure that we delete child directories before parent directories
        migration
            .rmdirs()
            .iter()
            .rev()
            .map(|dir_path| self.remove_directory(dir_path))
            .collect::<Result<Vec<()>, _>>()?;

        // here we create directories in order to ensure that we create parent directories before child directories
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
    ) -> anyhow::Result<()> {
        migration
            .actions()
            .get(&action)
            .ok_or_else(|| anyhow::anyhow!("Invalid action type"))?
            .iter()
            .map(|(file_path, entry)| {
                let path = self.path.join(file_path);

                if path.exists() {
                    let metadata = std::fs::metadata(&path).with_context(|| {
                        format!("Failed to get metadata for file: {:?}", file_path)
                    })?;

                    if metadata.is_dir() {
                        std::fs::remove_dir_all(&path).with_context(|| {
                            format!("Failed to remove existing directory: {:?}", file_path)
                        })?;
                    }

                    if metadata.is_file() {
                        std::fs::remove_file(&path)
                            .with_context(|| format!("Failed to remove file: {:?}", file_path))?;
                    }
                }

                match (&action, entry) {
                    (ActionType::Delete, None) => Ok(()),
                    (ActionType::Add | ActionType::Modify, Some(entry)) => {
                        // read blob data
                        let data = migration.load_blob_data(&entry.oid)?;

                        // TODO: use flag options
                        // open file as WRONLY, CREAT, EXCL using u32 flags
                        let mut file = std::fs::OpenOptions::new()
                            .write(true)
                            .create(true)
                            .truncate(true)
                            .open(path)
                            .with_context(|| format!("Failed to open file: {:?}", file_path))?;

                        // write data to file
                        file.write_all(data.as_bytes())
                            .with_context(|| format!("Failed to write to file: {:?}", file_path))?;

                        // update file mode if necessary
                        #[cfg(unix)]
                        {
                            use std::os::unix::fs::PermissionsExt;
                            let permissions = std::fs::Permissions::from_mode(entry.mode.as_u32());
                            std::fs::set_permissions(self.path.join(file_path), permissions)
                                .with_context(|| {
                                    format!("Failed to set permissions for file: {:?}", file_path)
                                })?;
                        }

                        Ok(())
                    }
                    _ => Err(anyhow::anyhow!("Invalid action and entry combination")),
                }
            })
            .collect::<Result<Vec<()>, _>>()?;

        Ok(())
    }

    fn remove_directory(&self, dir_path: &Path) -> anyhow::Result<()> {
        let dir_path = self.path.join(dir_path);

        std::fs::remove_dir_all(dir_path)?;

        Ok(())
    }

    fn make_directory(&self, dir_path: &Path) -> anyhow::Result<()> {
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
