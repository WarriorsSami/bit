use crate::areas::database::CommitCache;
use crate::areas::repository::Repository;
use crate::artifacts::branch::revision::Revision;
use crate::artifacts::checkout::migration::Migration;
use crate::artifacts::log::path_filter::PathFilter;
use crate::artifacts::merge::bca_finder::BCAFinder;

impl Repository {
    pub async fn merge(&mut self, target: &str, message: &str) -> anyhow::Result<()> {
        let current_ref = self.refs().current_ref(None)?;
        let head_oid = self
            .refs()
            .read_oid(&current_ref)?
            .ok_or_else(|| anyhow::anyhow!("no current HEAD to checkout from"))?;

        let target_revision = Revision::try_parse(target)?;
        let merge_oid = target_revision
            .resolve(self)?
            .ok_or_else(|| anyhow::anyhow!("target revision could not be resolved"))?;

        eprintln!(
            "Merging {} into {}",
            merge_oid.to_short_oid(),
            head_oid.to_short_oid()
        );

        // Find the best common ancestor
        let commit_cache = CommitCache::new();
        let database = self.database();

        let base_oid = {
            let best_common_ancestor_finder = BCAFinder::new(|oid| {
                commit_cache
                    .get_or_load_slim_commit(database, oid)
                    .expect("Failed to load commit")
            });
            best_common_ancestor_finder
                .find_best_common_ancestor(&head_oid, &merge_oid)
                .ok_or_else(|| {
                    anyhow::anyhow!("no common ancestor found between HEAD and target")
                })?
        };

        {
            let index = self.index();
            let mut index = index.lock().await;

            index.rehydrate()?;

            let tree_diff = self.database().tree_diff(
                Some(&base_oid),
                Some(&merge_oid),
                &PathFilter::empty(),
            )?;

            let mut migration = Migration::new_for_merge(self, &mut index, tree_diff);
            migration.apply_changes()?;

            index.write_updates()?;
        }

        let parents = vec![head_oid, merge_oid];
        let _ = self.write_commit(parents, message.to_string()).await?;

        Ok(())
    }
}
