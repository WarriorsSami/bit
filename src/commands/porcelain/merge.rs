use crate::areas::refs::HEAD_REF_NAME;
use crate::areas::repository::Repository;
use crate::artifacts::checkout::migration::Migration;
use crate::artifacts::log::path_filter::PathFilter;
use crate::artifacts::merge::inputs::MergeInputs;
use crate::artifacts::merge::resolution::MergeResolution;

// TODO: pattern match the merge type (null, fast-forward, normal) and handle each case separately
impl Repository {
    pub async fn merge(&mut self, target: &str, message: &str) -> anyhow::Result<()> {
        let merge_inputs = MergeInputs::new(self, HEAD_REF_NAME, target)?;

        if self.is_null_merge(&merge_inputs) {
            self.handle_null_merge(&merge_inputs)?;
            return Ok(());
        }

        if self.is_fast_forward_merge(&merge_inputs) {
            self.handle_fast_forward_merge(&merge_inputs).await?;
            return Ok(());
        }

        let conflicted = self.resolve_merge(&merge_inputs, target).await?;

        if !conflicted.is_empty() {
            self.refs().write_merge_head(merge_inputs.right_oid())?;
            self.refs().write_merge_msg(message)?;
            let names: Vec<String> = conflicted.iter().map(|p| p.display().to_string()).collect();
            anyhow::bail!(
                "Merge conflict in: {} — fix conflicts then commit",
                names.join(", ")
            );
        }

        let parents = vec![
            merge_inputs.left_oid().clone(),
            merge_inputs.right_oid().clone(),
        ];

        self.write_commit(parents, message.to_string()).await?;

        Ok(())
    }

    fn is_null_merge(&self, merge_inputs: &MergeInputs) -> bool {
        merge_inputs.base_oid() == merge_inputs.right_oid()
    }

    fn handle_null_merge(&self, merge_inputs: &MergeInputs) -> anyhow::Result<()> {
        writeln!(
            self.writer(),
            "Already up to date: {} is an ancestor of {}",
            merge_inputs.right_oid().to_short_oid(),
            merge_inputs.left_oid().to_short_oid()
        )?;
        Ok(())
    }

    fn is_fast_forward_merge(&self, merge_inputs: &MergeInputs) -> bool {
        merge_inputs.base_oid() == merge_inputs.left_oid()
    }

    async fn handle_fast_forward_merge(
        &self,
        merge_inputs: &MergeInputs<'_>,
    ) -> anyhow::Result<()> {
        let short_left_oid = merge_inputs.left_oid().to_short_oid();
        let short_right_oid = merge_inputs.right_oid().to_short_oid();

        writeln!(
            self.writer(),
            "Fast-forwarding {} to {}",
            short_left_oid,
            short_right_oid
        )?;

        let index = self.index();
        let mut index = index.lock().await;

        index.rehydrate()?;

        let tree_diff = self.database().tree_diff(
            Some(merge_inputs.left_oid()),
            Some(merge_inputs.right_oid()),
            &PathFilter::empty(),
        )?;

        let mut migration = Migration::new_for_merge(self, &mut index, tree_diff);
        migration.apply_changes()?;

        index.write_updates()?;
        self.refs().update_head(merge_inputs.right_oid().clone())?;

        Ok(())
    }

    /// Returns the list of conflicted paths (empty = clean merge)
    async fn resolve_merge(
        &self,
        merge_inputs: &MergeInputs<'_>,
        right_name: &str,
    ) -> anyhow::Result<Vec<std::path::PathBuf>> {
        let index = self.index();
        let mut index = index.lock().await;

        index.rehydrate()?;

        let merge_resolution = MergeResolution::new(self, merge_inputs);
        merge_resolution.execute(&mut index, right_name)?;

        let conflicted = index.conflicted_paths();
        index.write_updates()?;

        Ok(conflicted)
    }
}
