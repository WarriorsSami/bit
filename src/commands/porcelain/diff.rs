use crate::domain::areas::index::Index;
use crate::domain::areas::repository::Repository;
use crate::domain::areas::workspace::Workspace;
use crate::domain::objects::file_change::{FileChangeType, WorkspaceChangeType};
use crate::domain::objects::object::Object;
use crate::domain::objects::object_id::ObjectId;
use crate::domain::objects::status::FileStatSet;
use derive_new::new;
use std::path::{Path, PathBuf};

const NULL_OID_RAW: &str = "0000000000000000000000000000000000000000";
const NULL_PATH: &str = "/dev/null";

impl Repository {
    pub async fn diff(&mut self, _cached: bool) -> anyhow::Result<()> {
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
                WorkspaceChangeType::Modified => self.print_diff(
                    &mut DiffTarget::from_index(file, &index)?,
                    &mut DiffTarget::from_file(file, self.workspace(), &status_info.file_stats)?,
                ),
                WorkspaceChangeType::Deleted => self.print_diff(
                    &mut DiffTarget::from_index(file, &index)?,
                    &mut DiffTarget::from_nothing(file)?,
                ),
                _ => unreachable!(),
            })
            .collect::<anyhow::Result<Vec<_>>>()?;

        Ok(())
    }

    fn print_diff(&self, a: &mut DiffTarget, b: &mut DiffTarget) -> anyhow::Result<()> {
        if a.oid == b.oid && a.mode == b.mode {
            return Ok(());
        }

        a.file = Path::new("a").join(&a.file);
        b.file = Path::new("b").join(&b.file);

        writeln!(
            self.writer(),
            "diff --git {} {}",
            a.file.display(),
            b.file.display()
        )?;
        self.print_diff_mode(a, b)?;
        self.print_diff_content(a, b)?;

        Ok(())
    }

    fn print_diff_mode(&self, a: &DiffTarget, b: &DiffTarget) -> anyhow::Result<()> {
        if b.mode.is_none() {
            writeln!(self.writer(), "deleted file mode {}", a.pretty_mode())?;
        } else if a.mode != b.mode {
            writeln!(self.writer(), "old mode {}", a.pretty_mode())?;
            writeln!(self.writer(), "new mode {}", b.pretty_mode())?;
        }

        Ok(())
    }

    fn print_diff_content(&self, a: &DiffTarget, b: &DiffTarget) -> anyhow::Result<()> {
        if a.oid == b.oid {
            return Ok(());
        }

        let mut oid_range = format!("index {}..{}", a.oid.to_short_oid(), b.oid.to_short_oid());
        if a.mode == b.mode {
            oid_range.push_str(format!(" {}", a.pretty_mode()).as_str());
        }

        writeln!(self.writer(), "{oid_range}")?;
        writeln!(self.writer(), "--- {}", a.diff_path().display())?;
        writeln!(self.writer(), "+++ {}", b.diff_path().display())?;

        Ok(())
    }
}

#[derive(Debug, Clone, new)]
struct DiffTarget<'d> {
    file: PathBuf,
    oid: ObjectId,
    mode: Option<&'d str>,
}

impl<'d> DiffTarget<'d> {
    pub fn from_index(file: &Path, index: &'d Index) -> anyhow::Result<Self> {
        match index.entry_by_path(file) {
            Some(entry) => {
                let oid = &entry.oid;
                let mode = entry.metadata.mode.as_str();

                Ok(Self {
                    file: file.to_path_buf(),
                    oid: oid.clone(),
                    mode: Some(mode),
                })
            }
            None => anyhow::bail!("File {} not tracked", file.display()),
        }
    }

    pub fn from_file(
        file: &Path,
        workspace: &Workspace,
        file_stats: &'d FileStatSet,
    ) -> anyhow::Result<Self> {
        let blob = workspace.parse_blob(file)?;
        let oid = blob.object_id()?;
        let mode = file_stats
            .get(file)
            .ok_or_else(|| anyhow::anyhow!("File {} not tracked", file.display()))?
            .mode
            .as_str();

        Ok(Self {
            file: file.to_path_buf(),
            oid,
            mode: Some(mode),
        })
    }

    pub fn from_nothing(file: &Path) -> anyhow::Result<Self> {
        Ok(Self {
            file: file.to_path_buf(),
            oid: ObjectId::try_parse(NULL_OID_RAW.to_string())?,
            mode: None,
        })
    }

    pub fn diff_path(&self) -> PathBuf {
        if self.mode.is_some() {
            self.file.clone()
        } else {
            Path::new(NULL_PATH).to_path_buf()
        }
    }

    pub fn pretty_mode(&self) -> &'d str {
        if let Some(mode) = self.mode {
            mode
        } else {
            "100644"
        }
    }
}
