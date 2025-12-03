use crate::areas::database::Database;
use crate::areas::index::Index;
use crate::areas::workspace::Workspace;
use crate::artifacts::objects::object::Object;
use crate::artifacts::objects::object_id::ObjectId;
use crate::artifacts::status::status_info::{FileStatSet, HeadTree};
use derive_new::new;
use std::path::{Path, PathBuf};

const NULL_OID_RAW: &str = "0000000000000000000000000000000000000000";
const NULL_PATH: &str = "/dev/null";

pub type LineSet = Vec<String>;

#[derive(Debug, Clone, new)]
pub struct DiffTarget<'d> {
    pub(crate) file: PathBuf,
    pub(crate) oid: ObjectId,
    pub(crate) mode: Option<&'d str>,
    pub(crate) data: LineSet,
}

impl<'d> DiffTarget<'d> {
    pub fn from_head(
        file: &Path,
        head_tree: &'d HeadTree,
        database: &'d Database,
    ) -> anyhow::Result<Self> {
        head_tree
            .get(file)
            .map(|entry| {
                let oid = &entry.oid;
                let mode = entry.mode.as_str();
                let blob = database.parse_object_as_blob(oid)?;
                let blob =
                    blob.ok_or_else(|| anyhow::anyhow!("File {} not tracked", file.display()))?;

                Ok(Self {
                    file: file.to_path_buf(),
                    oid: oid.clone(),
                    mode: Some(mode),
                    data: blob.content().lines().map(|s| s.to_string()).collect(),
                })
            })
            .unwrap_or_else(|| anyhow::bail!("File {} not tracked", file.display()))
    }

    pub fn from_index(
        file: &Path,
        index: &'d Index,
        database: &'d Database,
    ) -> anyhow::Result<Self> {
        index
            .entry_by_path(file)
            .map(|entry| {
                let oid = &entry.oid;
                let mode = entry.metadata.mode.as_str();
                let blob = database.parse_object_as_blob(oid)?;
                let blob =
                    blob.ok_or_else(|| anyhow::anyhow!("File {} not tracked", file.display()))?;

                Ok(Self {
                    file: file.to_path_buf(),
                    oid: oid.clone(),
                    mode: Some(mode),
                    data: blob.content().lines().map(|s| s.to_string()).collect(),
                })
            })
            .unwrap_or_else(|| anyhow::bail!("File {} not tracked", file.display()))
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
            data: blob.content().lines().map(|s| s.to_string()).collect(),
        })
    }

    pub fn from_nothing(file: &Path) -> anyhow::Result<Self> {
        Ok(Self {
            file: file.to_path_buf(),
            oid: ObjectId::try_parse(NULL_OID_RAW.to_string())?,
            mode: None,
            data: Vec::new(),
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
