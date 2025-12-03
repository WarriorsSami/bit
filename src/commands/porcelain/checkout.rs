use crate::areas::repository::Repository;
use crate::artifacts::branch::revision::RevisionContext;
use crate::artifacts::diff::migration::Migration;
use crate::artifacts::diff::tree_diff::TreeDiff;
use std::path::Path;

impl Repository {
    pub async fn checkout(&mut self, target: &str) -> anyhow::Result<()> {
        let current_oid = self
            .refs()
            .read_head()?
            .ok_or_else(|| anyhow::anyhow!("no current HEAD to checkout from"))?;

        // TODO: extract to common utility
        let revision_context = RevisionContext::new(self);
        let target_revision = RevisionContext::try_parse(target)?;
        let target_oid = revision_context
            .resolve(target_revision)?
            .ok_or_else(|| anyhow::anyhow!("target revision could not be resolved"))?;

        let index = self.index();
        let mut index = index.lock().await;

        index.rehydrate()?;

        // TODO: extract to common utility
        let mut tree_diff = TreeDiff::new(self);
        tree_diff.compare_oids(Some(current_oid), Some(target_oid.clone()), Path::new(""))?;

        let mut migration = Migration::new(self, &mut index, tree_diff);
        migration.apply_changes()?;

        index.write_updates()?;
        self.refs().update_head(target_oid)?;

        Ok(())
    }
}
