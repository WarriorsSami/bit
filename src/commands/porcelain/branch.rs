use crate::domain::areas::repository::Repository;
use crate::domain::objects::branch_name::BranchName;

impl Repository {
    pub fn branch(
        &mut self,
        branch_name: &str,
        _source_commit: Option<&str>,
    ) -> anyhow::Result<()> {
        let branch_name = BranchName::try_parse(branch_name.to_string())?;
        self.refs().create_branch(branch_name)?;

        Ok(())
    }
}
