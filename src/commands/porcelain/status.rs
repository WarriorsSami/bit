use crate::areas::repository::Repository;
use crate::artifacts::status::file_change::FileChangeType;
use crate::artifacts::status::status_info::StatusInfo;
use colored::*;
use std::collections::BTreeMap;
use std::path::PathBuf;

// Terminology:
// - untracked files: files that are not tracked by the index
// - workspace modified files: files that are tracked by the index but have changes in the workspace
// - workspace deleted files: files that are tracked by the index but have been deleted from the workspace
// - index added files: files that are in the index but not in the HEAD commit
// - index modified files: files that are in the index and in the HEAD commit but have different content or mode
// - index deleted files: files that are in the HEAD commit but not in the index
impl Repository {
    pub async fn display_status(&mut self, porcelain: bool) -> anyhow::Result<()> {
        let index = self.index();
        let mut index = index.lock().await;

        index.rehydrate()?;
        let status_info = self.status().initialize(&mut index).await?;
        index.write_updates()?;

        if porcelain {
            status_info.changed_files.iter().for_each(|(file, status)| {
                writeln!(self.writer(), "{} {}", status, file.display()).unwrap();
            });

            status_info.untracked_files.iter().for_each(|file| {
                writeln!(self.writer(), "?? {}", file.display()).unwrap();
            });
        } else {
            self.print_changes("Changes to be committed", &status_info.index_changeset)?;
            self.print_changes(
                "Changes not staged for commit",
                &status_info.workspace_changeset,
            )?;
            self.print_changes("Untracked files", &status_info.untracked_changeset)?;

            self.print_commit_status(&status_info)?;
        }

        Ok(())
    }

    fn print_changes(
        &self,
        message: &str,
        changeset: &BTreeMap<PathBuf, FileChangeType>,
    ) -> anyhow::Result<()> {
        if !changeset.is_empty() {
            writeln!(self.writer(), "{}:\n", message.bold())?;
            for (file, change) in changeset {
                writeln!(
                    self.writer(),
                    "{}{}",
                    change,
                    file.display().to_string().cyan()
                )?;
            }
            writeln!(self.writer())?;
        }

        Ok(())
    }

    fn print_commit_status(&self, status_info: &StatusInfo) -> anyhow::Result<()> {
        if !status_info.index_changeset.is_empty() {
            return Ok(());
        }

        if !status_info.workspace_changeset.is_empty() {
            writeln!(self.writer(), "{}", "no changes added to commit".yellow())?;
            return Ok(());
        }

        if !status_info.untracked_changeset.is_empty() {
            writeln!(
                self.writer(),
                "{}",
                "no changes added to commit but untracked files present".yellow()
            )?;
            return Ok(());
        }

        writeln!(
            self.writer(),
            "{}",
            "nothing to commit, working tree clean".green()
        )?;
        Ok(())
    }
}
