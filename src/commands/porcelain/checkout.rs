use crate::areas::repository::Repository;
use crate::artifacts::branch::branch_name::SymRefName;
use crate::artifacts::branch::revision::Revision;
use crate::artifacts::checkout::migration::Migration;
use crate::artifacts::log::path_filter::PathFilter;
use crate::artifacts::objects::object_id::ObjectId;

const DETACHMENT_NOTICE: &str = r#"
You are in 'detached HEAD' state. You can look around, make experimental
changes and commit them, and you can discard any commits you make in this
state without impacting any branches by performing another checkout.

If you want to create a new branch to retain commits you create, you may
do so (now or later) by using the branch command. Example:

    bit branch <new-branch-name>
"#;

impl Repository {
    pub async fn checkout(&mut self, target: &str) -> anyhow::Result<()> {
        let current_ref = self.refs().current_ref(None)?;
        let current_oid = self
            .refs()
            .read_oid(&current_ref)?
            .ok_or_else(|| anyhow::anyhow!("no current HEAD to checkout from"))?;

        let target_revision = Revision::try_parse(target)?;
        let target_oid = target_revision
            .resolve(self)?
            .ok_or_else(|| anyhow::anyhow!("target revision could not be resolved"))?;

        let index = self.index();
        let mut index = index.lock().await;

        index.rehydrate()?;

        let tree_diff = self.database().tree_diff(
            Some(&current_oid),
            Some(&target_oid),
            PathFilter::empty(),
        )?;

        let mut migration = Migration::new(self, &mut index, tree_diff);
        migration.apply_changes()?;

        index.write_updates()?;
        self.refs()
            .set_head(target, target_oid.clone().as_ref().into())?;
        let new_ref = self.refs().current_ref(None)?;

        self.print_previous_head(&current_ref, &current_oid, &target_oid)?;
        self.print_detachment_notice(&current_ref, &new_ref, target)?;
        self.print_new_head(&current_ref, &new_ref, &target_oid, target)?;

        Ok(())
    }

    fn print_previous_head(
        &self,
        current_ref: &SymRefName,
        current_oid: &ObjectId,
        target_oid: &ObjectId,
    ) -> anyhow::Result<()> {
        if current_ref.is_detached_head() && current_oid != target_oid {
            self.print_head_position("Previous HEAD position was", current_oid)?;
        }

        Ok(())
    }

    fn print_detachment_notice(
        &self,
        current_ref: &SymRefName,
        new_ref: &SymRefName,
        target: &str,
    ) -> anyhow::Result<()> {
        if !current_ref.is_detached_head() && new_ref.is_detached_head() {
            eprintln!("Note: checking out '{}'.\n{}", target, DETACHMENT_NOTICE);
        }

        Ok(())
    }

    fn print_new_head(
        &self,
        current_ref: &SymRefName,
        new_ref: &SymRefName,
        target_oid: &ObjectId,
        target: &str,
    ) -> anyhow::Result<()> {
        if new_ref.is_detached_head() {
            self.print_head_position("HEAD is now at", target_oid)?;
        } else if new_ref == current_ref {
            eprintln!("Already on '{}'", target);
        } else {
            eprintln!("Switched to branch '{}'", target);
        }

        Ok(())
    }

    fn print_head_position(&self, message: &str, oid: &ObjectId) -> anyhow::Result<()> {
        let commit = self
            .database()
            .parse_object_as_commit(oid)?
            .ok_or_else(|| anyhow::anyhow!("object is not a commit"))?;
        let short_oid = oid.to_short_oid();

        eprintln!("{} {} {}", message, short_oid, commit.short_message());
        Ok(())
    }
}
