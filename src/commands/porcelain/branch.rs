use crate::domain::areas::repository::Repository;
use crate::domain::objects::branch_name::BranchName;
use crate::domain::objects::revision::RevisionContext;

impl Repository {
    pub fn branch(
        &mut self,
        branch_name: &str,
        source_revision: Option<&str>,
    ) -> anyhow::Result<()> {
        let branch_name = BranchName::try_parse(branch_name.to_string())?;

        let source_oid = if let Some(source_revision) = source_revision {
            let (revision_context, parsed_revision) =
                RevisionContext::initialize(self, source_revision)?;
            revision_context.resolve(parsed_revision)?
        } else {
            self.refs().read_head()?
        }
        .ok_or_else(|| anyhow::anyhow!("no current HEAD to branch from"))?;

        self.refs().create_branch(branch_name, source_oid)?;

        Ok(())
    }
}
