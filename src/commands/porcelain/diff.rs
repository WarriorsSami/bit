use crate::areas::index::Index;
use crate::areas::repository::Repository;
use crate::areas::workspace::Workspace;
use crate::artifacts::branch::revision::Revision;
use crate::artifacts::diff::diff_algorithm::{DiffAlgorithm, Hunk, MyersDiff};
use crate::artifacts::diff::diff_target::DiffTarget;
use crate::artifacts::diff::tree_diff::DiffFilter;
use crate::artifacts::log::path_filter::PathFilter;
use crate::artifacts::objects::object_id::ObjectId;
use crate::artifacts::status::file_change::{FileChangeType, IndexChangeType, WorkspaceChangeType};
use crate::artifacts::status::status_info::StatusInfo;
use colored::Colorize;
use std::path::Path;

impl Repository {
    pub async fn diff(
        &mut self,
        cached: bool,
        name_status: bool,
        diff_filter: Option<&str>,
        old_revision: Option<&str>,
        new_revision: Option<&str>,
    ) -> anyhow::Result<()> {
        // If both commits are provided, compare them
        if let (Some(old_revision), Some(new_revision)) = (old_revision, new_revision) {
            // parse revisions to commit OIDs
            let commit_a = Revision::try_parse(old_revision)?
                .resolve(self)?
                .ok_or_else(|| {
                    anyhow::anyhow!("Old revision could not be resolved: {}", old_revision)
                })?;
            let commit_b = Revision::try_parse(new_revision)?
                .resolve(self)?
                .ok_or_else(|| {
                    anyhow::anyhow!("New revision could not be resolved: {}", new_revision)
                })?;

            // parse raw diff filter to DiffFilter
            let diff_filter = if let Some(filter) = diff_filter {
                DiffFilter::try_parse(filter)
            } else {
                None
            };

            return self.diff_commits(commit_a, commit_b, name_status, diff_filter);
        }

        let index = self.index();
        let mut index = index.lock().await;

        index.rehydrate()?;
        let status_info = self.status().initialize(&mut index).await?;

        if !cached {
            self.diff_index_workspace(&status_info, &index, self.workspace())?;
        } else {
            self.diff_head_index(&status_info, &index)?;
        }

        Ok(())
    }

    fn diff_commits(
        &self,
        commit_a: ObjectId,
        commit_b: ObjectId,
        name_status: bool,
        diff_filter: Option<DiffFilter>,
    ) -> anyhow::Result<()> {
        let tree_diff =
            self.database()
                .tree_diff(Some(&commit_a), Some(&commit_b), &PathFilter::empty())?;
        let changeset = tree_diff.changes();

        for (path, change_type) in changeset {
            // Apply diff filter if specified
            if let Some(filter) = diff_filter
                && !change_type.matches_filter(filter)
            {
                continue;
            }

            if name_status {
                // Print in name-status format: <status>\t<path>
                writeln!(
                    self.writer(),
                    "{}\t{}",
                    change_type.status_char(),
                    path.display()
                )?;
            } else {
                let (old_entry, new_entry) = tree_diff.get_entries(path);
                self.print_diff(
                    &mut DiffTarget::from_entry(path, old_entry, self.database())?,
                    &mut DiffTarget::from_entry(path, new_entry, self.database())?,
                )?;
            }
        }

        Ok(())
    }

    fn diff_index_workspace(
        &self,
        status_info: &StatusInfo,
        index: &Index,
        workspace: &Workspace,
    ) -> anyhow::Result<()> {
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
                    &mut DiffTarget::from_index(file, index, self.database())?,
                    &mut DiffTarget::from_file(file, workspace, &status_info.file_stats)?,
                ),
                WorkspaceChangeType::Deleted => self.print_diff(
                    &mut DiffTarget::from_index(file, index, self.database())?,
                    &mut DiffTarget::from_nothing(file)?,
                ),
                _ => unreachable!(),
            })
            .collect::<anyhow::Result<Vec<_>>>()?;

        Ok(())
    }

    fn diff_head_index(&self, status_info: &StatusInfo, index: &Index) -> anyhow::Result<()> {
        status_info
            .index_changeset
            .iter()
            .filter_map(|(file, change)| match *change {
                FileChangeType::Index(IndexChangeType::Added) => {
                    Some((file, IndexChangeType::Added))
                }
                FileChangeType::Index(IndexChangeType::Modified) => {
                    Some((file, IndexChangeType::Modified))
                }
                FileChangeType::Index(IndexChangeType::Deleted) => {
                    Some((file, IndexChangeType::Deleted))
                }
                _ => None,
            })
            .map(|(file, change)| match change {
                IndexChangeType::Added => self.print_diff(
                    &mut DiffTarget::from_nothing(file)?,
                    &mut DiffTarget::from_index(file, index, self.database())?,
                ),
                IndexChangeType::Modified => self.print_diff(
                    &mut DiffTarget::from_head(file, &status_info.head_tree, self.database())?,
                    &mut DiffTarget::from_index(file, index, self.database())?,
                ),
                IndexChangeType::Deleted => self.print_diff(
                    &mut DiffTarget::from_head(file, &status_info.head_tree, self.database())?,
                    &mut DiffTarget::from_nothing(file)?,
                ),
                _ => unreachable!(),
            })
            .collect::<anyhow::Result<Vec<_>>>()?;

        Ok(())
    }

    pub fn print_diff(&self, a: &mut DiffTarget, b: &mut DiffTarget) -> anyhow::Result<()> {
        if a.oid == b.oid && a.mode == b.mode {
            return Ok(());
        }

        a.file = Path::new("a").join(&a.file);
        b.file = Path::new("b").join(&b.file);

        writeln!(
            self.writer(),
            "{}",
            format!("diff --git {} {}", a.file.display(), b.file.display()).bold()
        )?;
        self.print_diff_mode(a, b)?;
        self.print_diff_content(a, b)?;

        Ok(())
    }

    fn print_diff_mode(&self, a: &DiffTarget, b: &DiffTarget) -> anyhow::Result<()> {
        if a.mode.is_none() {
            writeln!(
                self.writer(),
                "{}",
                format!("new file mode {}", b.pretty_mode()).bold()
            )?;
        } else if b.mode.is_none() {
            writeln!(
                self.writer(),
                "{}",
                format!("deleted file mode {}", a.pretty_mode()).bold()
            )?;
        } else if a.mode != b.mode {
            writeln!(
                self.writer(),
                "{}",
                format!("old mode {}", a.pretty_mode()).bold()
            )?;
            writeln!(
                self.writer(),
                "{}",
                format!("new mode {}", b.pretty_mode()).bold()
            )?;
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

        writeln!(self.writer(), "{}", oid_range.to_string().bold())?;
        writeln!(
            self.writer(),
            "{}",
            format!("--- {}", a.diff_path().display()).bold()
        )?;
        writeln!(
            self.writer(),
            "{}",
            format!("+++ {}", b.diff_path().display()).bold()
        )?;

        let hunks = MyersDiff::new(&a.data, &b.data).flatten_diff();
        for hunk in hunks {
            self.print_diff_hunk(&hunk)?;
        }

        Ok(())
    }

    fn print_diff_hunk(&self, hunk: &Hunk<String>) -> anyhow::Result<()> {
        let a_offset = format!("{},{}", hunk.a_start(), hunk.a_size());
        let b_offset = format!("{},{}", hunk.b_start(), hunk.b_size());

        writeln!(
            self.writer(),
            "{}",
            format!("@@ -{a_offset} +{b_offset} @@").cyan()
        )?;

        for edit in hunk.edits() {
            writeln!(self.writer(), "{}", edit)?;
        }

        Ok(())
    }
}
