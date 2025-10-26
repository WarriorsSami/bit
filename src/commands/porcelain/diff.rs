use crate::domain::areas::index::Index;
use crate::domain::areas::repository::Repository;
use crate::domain::objects::file_change::{FileChangeType, WorkspaceChangeType};
use crate::domain::objects::object::Object;
use crate::domain::objects::object_id::ObjectId;
use crate::domain::objects::status::FileStatSet;
use std::path::Path;

const NULL_OID_RAW: &str = "0000000000000000000000000000000000000000";
const NULL_PATH: &str = "/dev/null";

impl Repository {
    pub async fn diff(&mut self, cached: bool) -> anyhow::Result<()> {
        let index = self.index();
        let mut index = index.lock().await;

        index.rehydrate()?;
        let status_info = self.status().initialize(&mut index).await?;

        status_info
            .workspace_changeset
            .iter()
            .filter_map(|(file, change)| match *change {
                FileChangeType::Workspace(WorkspaceChangeType::Modified) => {
                    Some((file, WorkspaceChangeType::Modified))
                }
                FileChangeType::Workspace(WorkspaceChangeType::Deleted) => {
                    Some((file, WorkspaceChangeType::Deleted))
                }
                _ => None,
            })
            .map(|(file, change)| match change {
                WorkspaceChangeType::Modified => {
                    self.diff_file_modified(file, cached, &index, &status_info.file_stats)
                }
                WorkspaceChangeType::Deleted => self.diff_file_deleted(file, cached, &index),
                _ => unreachable!(),
            })
            .collect::<anyhow::Result<Vec<_>>>()?;

        Ok(())
    }

    fn diff_file_modified(
        &self,
        file: &Path,
        _cached: bool,
        index: &Index,
        file_stats: &FileStatSet,
    ) -> anyhow::Result<()> {
        match index.entry_by_path(file) {
            Some(entry) => {
                let a_oid = &entry.oid;
                let a_mode = entry.metadata.mode.as_str();
                let a_path = Path::new("a").join(file);

                let blob = self.workspace().parse_blob(file)?;
                let b_oid = blob.object_id()?;
                // if there is no file stat, report the error and return
                let b_mode = file_stats
                    .get(file)
                    .ok_or_else(|| anyhow::anyhow!("File {} not tracked", file.display()))?
                    .mode
                    .as_str();
                let b_path = Path::new("b").join(file);

                writeln!(
                    self.writer(),
                    "diff --git {} {}",
                    a_path.display(),
                    b_path.display()
                )?;

                if a_mode != b_mode {
                    writeln!(self.writer(), "old mode {}", a_mode)?;
                    writeln!(self.writer(), "new mode {}", b_mode)?;
                }

                if *a_oid == b_oid {
                    return Ok(());
                }

                let mut oid_range =
                    format!("index {}..{}", a_oid.to_short_oid(), b_oid.to_short_oid(),);
                if a_mode == b_mode {
                    oid_range.push_str(format!(" {}", a_mode).as_str());
                }

                writeln!(self.writer(), "{oid_range}")?;
                writeln!(self.writer(), "--- {}", a_path.display())?;
                writeln!(self.writer(), "+++ {}", b_path.display())?;

                Ok(())
            }
            None => anyhow::bail!("File {} not tracked", file.display()),
        }
    }

    fn diff_file_deleted(&self, file: &Path, _cached: bool, index: &Index) -> anyhow::Result<()> {
        match index.entry_by_path(file) {
            Some(entry) => {
                let a_oid = &entry.oid;
                let a_mode = entry.metadata.mode.as_str();
                let a_path = Path::new("a").join(file);

                let b_oid = ObjectId::try_parse(NULL_OID_RAW.to_string())?;
                let b_path = Path::new("b").join(file);

                writeln!(
                    self.writer(),
                    "diff --git {} {}",
                    a_path.display(),
                    b_path.display()
                )?;
                writeln!(self.writer(), "deleted file mode {}", a_mode)?;

                let oid_range = format!("index {}..{}", a_oid.to_short_oid(), b_oid.to_short_oid());

                writeln!(self.writer(), "{oid_range}")?;
                writeln!(self.writer(), "--- {}", a_path.display())?;
                writeln!(self.writer(), "+++ {}", NULL_PATH)?;

                Ok(())
            }
            None => anyhow::bail!("File {} not tracked", file.display()),
        }
    }
}
