use crate::areas::index::Index;
use crate::areas::repository::Repository;
use crate::artifacts::database::database_entry::DatabaseEntry;
use crate::artifacts::index::index_entry::{EntryMetadata, IndexEntry};
use crate::artifacts::objects::object::Object;
use crate::artifacts::status::file_change::{IndexChangeType, WorkspaceChangeType};
use derive_new::new;
use std::path::Path;

#[derive(new)]
pub struct Inspector<'r> {
    repository: &'r Repository,
}

impl<'r> Inspector<'r> {
    pub fn is_indirectly_tracked(&self, path: &Path, index: &Index) -> anyhow::Result<bool> {
        if path.is_file() {
            return Ok(index.is_directly_tracked(path));
        }

        let paths = self.repository.workspace().list_dir(Some(path))?;
        let files = paths.iter().filter(|p| p.is_file());
        let dirs = paths.iter().filter(|p| p.is_dir());

        let mut paths = files.chain(dirs);

        // chain the iterators and check if any of the files or directories are indirectly tracked
        if paths.clone().count() == 0 {
            Ok(true)
        } else {
            Ok(paths.any(|p| self.is_indirectly_tracked(p, index).unwrap_or(false)))
        }
    }

    fn is_content_changed(&self, index_entry: &IndexEntry) -> anyhow::Result<bool> {
        let blob = self.repository.workspace().parse_blob(&index_entry.name)?;
        let oid = blob.object_id()?;

        Ok(oid != index_entry.oid)
    }

    pub fn check_index_against_workspace(
        &self,
        entry: Option<&IndexEntry>,
        stat: Option<&EntryMetadata>,
    ) -> anyhow::Result<WorkspaceChangeType> {
        match (entry, stat) {
            (None, _) => Ok(WorkspaceChangeType::Untracked),
            (Some(_), None) => Ok(WorkspaceChangeType::Deleted),
            (Some(entry), Some(stat)) if !entry.stat_match(stat) => {
                Ok(WorkspaceChangeType::Modified)
            }
            (Some(entry), Some(stat)) if entry.stat_match(stat) && entry.times_match(stat) => {
                Ok(WorkspaceChangeType::None)
            }
            (Some(entry), Some(_)) if self.is_content_changed(entry)? => {
                Ok(WorkspaceChangeType::Modified)
            }
            _ => Ok(WorkspaceChangeType::None),
        }
    }

    pub fn check_index_against_head_tree(
        &self,
        index_entry: Option<&IndexEntry>,
        head_entry: Option<&DatabaseEntry>,
    ) -> IndexChangeType {
        match (index_entry, head_entry) {
            (Some(index_entry), Some(head_entry))
                if head_entry.mode != index_entry.metadata.mode
                    || head_entry.oid != index_entry.oid =>
            {
                IndexChangeType::Modified
            }
            (Some(_), None) => IndexChangeType::Added,
            (None, Some(_)) => IndexChangeType::Deleted,
            _ => IndexChangeType::None,
        }
    }
}
