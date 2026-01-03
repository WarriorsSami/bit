use crate::areas::refs::HEAD_REF_NAME;
use crate::areas::repository::Repository;
use crate::artifacts::branch::branch_name::SymRefName;
use crate::artifacts::branch::revision::Revision;
use crate::artifacts::diff::diff_target::DiffTarget;
use crate::artifacts::log::rev_list::RevList;
use crate::artifacts::objects::commit::Commit;
use crate::artifacts::objects::object::Object;
use crate::{CommitDecoration, CommitDisplayFormat};
use colored::Colorize;

const RANGE_REGEX: &str = r"^(?P<excluded>[^.]+)\.\.(?P<included>[^.]+)$";
const EXCLUDED_REGEX: &str = r"^\^(?P<excluded>.+)$";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LogTarget {
    IncludedRevision(Revision),
    RangeExpression {
        excluded: Revision,
        included: Revision,
    },
    ExcludedRevision(Revision),
}

pub fn parse_log_target(target: &str) -> anyhow::Result<LogTarget> {
    let range_regex = regex::Regex::new(RANGE_REGEX)?;
    let excluded_regex = regex::Regex::new(EXCLUDED_REGEX)?;

    if let Some(captures) = range_regex.captures(target) {
        let excluded_str = captures
            .name("excluded")
            .ok_or(anyhow::anyhow!(
                "Failed to parse excluded revision from range expression"
            ))?
            .as_str();
        let included_str = captures
            .name("included")
            .ok_or(anyhow::anyhow!(
                "Failed to parse included revision from range expression"
            ))?
            .as_str();

        let excluded = Revision::try_parse(excluded_str)?;
        let included = Revision::try_parse(included_str)?;

        Ok(LogTarget::RangeExpression { excluded, included })
    } else if let Some(captures) = excluded_regex.captures(target) {
        let excluded_str = captures
            .name("excluded")
            .ok_or(anyhow::anyhow!(
                "Failed to parse excluded revision from exclusion expression"
            ))?
            .as_str();
        let excluded = Revision::try_parse(excluded_str)?;

        Ok(LogTarget::ExcludedRevision(excluded))
    } else {
        let included = Revision::try_parse(target)?;

        Ok(LogTarget::IncludedRevision(included))
    }
}

#[derive(Debug, Clone)]
pub struct LogOptions {
    pub targets: Option<Vec<LogTarget>>,
    pub oneline: bool,
    pub abbrev_commit: bool,
    pub format: CommitDisplayFormat,
    pub decorate: CommitDecoration,
    pub patch: bool,
}

impl Repository {
    pub fn log(&self, opts: &LogOptions) -> anyhow::Result<()> {
        self.set_reverse_refs(self.refs().reverse_refs()?);
        self.set_current_ref(self.refs().current_ref(None)?);

        let log_targets = opts
            .targets
            .clone()
            .unwrap_or(vec![LogTarget::IncludedRevision(Revision::try_parse(
                HEAD_REF_NAME,
            )?)]);
        let rev_list = RevList::new(self, log_targets);

        match rev_list {
            Ok(rev_list) => {
                for commit in rev_list.into_iter() {
                    // Display the commit in medium format
                    self.show_commit(&commit, opts)?;
                    writeln!(self.writer())?;
                }
            }
            Err(_) => {
                // No commits to show
                writeln!(self.writer(), "No commits to show.")?;
            }
        }

        Ok(())
    }

    // TODO: define a RepositoryWriter trait to abstract over the writer using trait objects
    pub fn show_commit(&self, commit: &Commit, opts: &LogOptions) -> anyhow::Result<()> {
        if opts.oneline {
            self.show_commit_oneline(commit, true, CommitDecoration::Short)?;
        } else {
            match opts.format {
                CommitDisplayFormat::Medium => {
                    self.show_commit_medium(commit, opts.abbrev_commit, opts.decorate)?;
                }
                CommitDisplayFormat::OneLine => {
                    self.show_commit_oneline(commit, opts.abbrev_commit, opts.decorate)?;
                }
            }
        }

        self.show_commit_patch(commit, opts.patch)?;

        Ok(())
    }

    fn show_commit_patch(&self, commit: &Commit, patch: bool) -> anyhow::Result<()> {
        if !patch {
            return Ok(());
        }

        self.print_commit_diff(commit)?;

        Ok(())
    }

    fn print_commit_diff(&self, commit: &Commit) -> anyhow::Result<()> {
        let parent_oid = commit.parent();
        let commit_oid = commit.object_id()?;

        let tree_diff = self.database().tree_diff(parent_oid, Some(&commit_oid))?;
        let changeset = tree_diff.changes();

        for path in changeset.keys() {
            let (old_entry, new_entry) = tree_diff.get_entries(path);
            self.print_diff(
                &mut DiffTarget::from_entry(path, old_entry, self.database())?,
                &mut DiffTarget::from_entry(path, new_entry, self.database())?,
            )?;
        }

        Ok(())
    }

    fn show_commit_medium(
        &self,
        commit: &Commit,
        abbrev_commit: bool,
        decoration: CommitDecoration,
    ) -> anyhow::Result<()> {
        writeln!(
            self.writer(),
            "commit {}{}",
            self.abbrev_commit_id(commit, abbrev_commit)?.yellow(),
            self.commit_decoration(commit, decoration)?
        )?;
        writeln!(self.writer(), "Author: {}", commit.author().display_name())?;
        writeln!(
            self.writer(),
            "Date:   {}",
            commit.author().readable_timestamp()
        )?;
        writeln!(self.writer())?;
        for message_line in commit.message().lines() {
            writeln!(self.writer(), "    {}", message_line)?;
        }

        Ok(())
    }

    fn show_commit_oneline(
        &self,
        commit: &Commit,
        abbrev_commit: bool,
        decoration: CommitDecoration,
    ) -> anyhow::Result<()> {
        writeln!(
            self.writer(),
            "{}{} {}",
            self.abbrev_commit_id(commit, abbrev_commit)?.yellow(),
            self.commit_decoration(commit, decoration)?,
            commit.short_message()
        )?;

        Ok(())
    }

    fn commit_decoration(
        &self,
        commit: &Commit,
        decoration: CommitDecoration,
    ) -> anyhow::Result<String> {
        if decoration == CommitDecoration::None {
            return Ok(String::new());
        }

        let commit_oid = commit.object_id()?;
        if let Some(ref_names) = self.reverse_refs().get(&commit_oid) {
            let (head, refs): (Vec<_>, Vec<_>) = ref_names.iter().partition(|ref_name| {
                ref_name.is_detached_head() && !self.current_ref().is_detached_head()
            });
            let head = head.into_iter().cloned().collect::<Vec<_>>();
            let refs = refs.into_iter().cloned().collect::<Vec<_>>();

            let names = refs
                .into_iter()
                .map(|ref_name| {
                    self.ref_decoration_name(head.first().cloned(), ref_name, decoration)
                })
                .collect::<anyhow::Result<Vec<_>>>()?
                .join(", ");

            Ok(format!(" ({})", names))
        } else {
            Ok(String::new())
        }
    }

    fn ref_decoration_name(
        &self,
        head: Option<SymRefName>,
        ref_name: SymRefName,
        decoration: CommitDecoration,
    ) -> anyhow::Result<String> {
        let name = match decoration {
            CommitDecoration::Short => ref_name.to_short_name()?,
            CommitDecoration::Full => ref_name.as_ref().to_string(),
            CommitDecoration::None => unreachable!(),
        };
        let name = ref_name.to_colored_name(name)?;

        if let Some(head) = head
            && ref_name == *self.current_ref()
        {
            return Ok(head
                .to_colored_name(format!("{} -> {name}", head.as_ref()))?
                .to_string());
        }

        Ok(name)
    }

    fn abbrev_commit_id(&self, commit: &Commit, abbrev_commit: bool) -> anyhow::Result<String> {
        if abbrev_commit {
            Ok(commit.object_id()?.to_short_oid().as_str().to_string())
        } else {
            Ok(commit.object_id()?.as_ref().to_string())
        }
    }
}
