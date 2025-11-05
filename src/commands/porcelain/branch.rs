use crate::domain::areas::repository::Repository;

impl Repository {
    pub fn branch(
        &mut self,
        branch_name: &str,
        _source_commit: Option<&str>,
    ) -> anyhow::Result<()> {
        self.refs().create_branch(branch_name)?;

        Ok(())
    }
}
