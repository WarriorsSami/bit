use crate::BranchAction;
use crate::areas::repository::Repository;
use crate::artifacts::branch::branch_name::BranchName;
use crate::artifacts::branch::revision::Revision;
use colored::Colorize;

impl Repository {
    pub fn branch(&mut self, branch_action: &BranchAction) -> anyhow::Result<()> {
        match branch_action {
            BranchAction::Create {
                branch_name,
                source_refname,
            } => {
                let branch_name = BranchName::try_parse(branch_name.clone())?;

                let source_oid = if let Some(source_refname) = source_refname {
                    Revision::try_parse(source_refname.as_str())?.resolve(self)?
                } else {
                    self.refs().read_head()?
                }
                .ok_or_else(|| anyhow::anyhow!("no current HEAD to branch from"))?;

                self.refs().create_branch(branch_name, source_oid)?;
            }
            BranchAction::Delete {
                branch_names,
                force,
            } => {
                if !*force {
                    todo!("Implement merge safety checks before deleting branches");
                }

                for branch_name in branch_names {
                    let branch_name = BranchName::try_parse(branch_name.clone())?;
                    if self.refs().is_current_branch(&branch_name)? {
                        anyhow::bail!("cannot delete the current branch: {}", branch_name.as_ref());
                    }

                    let oid = self.refs().delete_branch(&branch_name)?;
                    let short_oid = oid.to_short_oid();

                    println!(
                        "Deleted branch {} (was {})",
                        branch_name.as_ref(),
                        short_oid
                    );
                }
            }
            BranchAction::List { verbose } => {
                let current_ref = self.refs().current_ref(None)?;
                let mut branches = self.refs().list_branches()?;
                branches.sort();

                let max_width = branches
                    .iter()
                    .map(|b| b.to_short_name())
                    .filter_map(|b| b.ok())
                    .map(|b| b.len())
                    .max()
                    .unwrap_or(0);

                for branch in branches {
                    let info = if branch == current_ref {
                        format!("* {}", branch.to_short_name()?)
                    } else {
                        format!("  {}", branch.to_short_name()?)
                    };

                    let extended_info = if *verbose {
                        let commit_oid = self.refs().read_oid(&branch)?.ok_or_else(|| {
                            anyhow::anyhow!("branch {} has no associated commit", branch.as_ref())
                        })?;
                        let commit = self
                            .database()
                            .parse_object_as_commit(&commit_oid)?
                            .ok_or_else(|| {
                                anyhow::anyhow!(
                                    "object {} is not a commit",
                                    commit_oid.to_short_oid()
                                )
                            })?;
                        let short_oid = commit_oid.to_short_oid();
                        let message = commit.short_message();

                        format!(
                            "{:width$} {} {}",
                            "",
                            short_oid,
                            message,
                            width = max_width - branch.to_short_name()?.len() + 1
                        )
                    } else {
                        "".to_string()
                    };
                    let branch_info = format!("{}{}", info, extended_info);

                    if branch == current_ref {
                        writeln!(self.writer(), "{}", branch_info.green())?;
                    } else {
                        writeln!(self.writer(), "{}", branch_info)?;
                    }
                }
            }
        }

        Ok(())
    }
}
