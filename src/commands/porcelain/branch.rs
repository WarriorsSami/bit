use crate::areas::repository::Repository;
use crate::artifacts::branch::branch_name::BranchName;
use crate::artifacts::branch::revision::RevisionContext;

impl Repository {
    pub fn branch(
        &mut self,
        branch_name: &str,
        source_refname: Option<&str>,
    ) -> anyhow::Result<()> {
        let branch_name = BranchName::try_parse(branch_name.to_string())?;

        let source_oid = if let Some(source_refname) = source_refname {
            let revision_context = RevisionContext::new(self);
            let revision = RevisionContext::try_parse(source_refname)?;

            revision_context.resolve(revision)?
        } else {
            self.refs().read_head()?
        }
        .ok_or_else(|| anyhow::anyhow!("no current HEAD to branch from"))?;

        self.refs().create_branch(branch_name, source_oid)?;

        Ok(())
    }
}
