use crate::domain::areas::index::Index;
use crate::domain::areas::repository::Repository;
use crate::domain::objects::database_entry::DatabaseEntry;
use crate::domain::objects::file_change::{IndexChangeType, WorkspaceChangeType};
use crate::domain::objects::index_entry::{EntryMetadata, IndexEntry};
use crate::domain::objects::object::Object;
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
        entry: &IndexEntry,
        stat: Option<&EntryMetadata>,
    ) -> anyhow::Result<WorkspaceChangeType> {
        match stat {
            Some(stat) if entry.stat_match(stat) => {
                if entry.times_match(stat) {
                    Ok(WorkspaceChangeType::None)
                } else if self.is_content_changed(entry)? {
                    Ok(WorkspaceChangeType::Modified)
                } else {
                    Ok(WorkspaceChangeType::None)
                }
            }
            Some(_) => Ok(WorkspaceChangeType::Modified),
            None => Ok(WorkspaceChangeType::Deleted),
        }
    }

    pub fn check_index_against_head_tree(
        &self,
        index_entry: &IndexEntry,
        head_entry: Option<&DatabaseEntry>,
    ) -> anyhow::Result<IndexChangeType> {
        match head_entry {
            Some(head_entry)
                if head_entry.mode != index_entry.metadata.mode
                    || head_entry.oid != index_entry.oid =>
            {
                Ok(IndexChangeType::Modified)
            }
            Some(_) => Ok(IndexChangeType::None),
            None => Ok(IndexChangeType::Added),
        }
    }
}
