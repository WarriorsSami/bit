use crate::CommitDisplayFormat;
use crate::areas::repository::Repository;

#[derive(Debug, Clone)]
pub struct LogOptions {
    pub oneline: bool,
    pub abbrev_commit: bool,
    pub format: CommitDisplayFormat,
}

impl Repository {
    pub fn log(&self, opts: &LogOptions) -> anyhow::Result<()> {
        let mut curr_commit_oid = self.refs().read_head()?;

        while let Some(commit_oid) = curr_commit_oid {
            let commit = self
                .database()
                .parse_object_as_commit(&commit_oid)?
                .ok_or_else(|| {
                    anyhow::anyhow!("Commit object not found: {}", commit_oid.as_ref())
                })?;

            // Display the commit in medium format
            commit.display(self, opts)?;
            writeln!(self.writer())?;

            // Move to the parent commit for the next iteration
            curr_commit_oid = commit.parent().cloned();
        }

        Ok(())
    }
}
