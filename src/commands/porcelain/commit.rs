use crate::areas::repository::Repository;
use crate::artifacts::objects::object::Object;
use std::io::Write;

impl Repository {
    pub async fn commit(&mut self, message: Option<&str>) -> anyhow::Result<()> {
        let message = match message {
            Some(m) => m.trim().to_string(),
            None => self.refs().read_merge_msg()?.ok_or_else(|| {
                anyhow::anyhow!(
                    "no commit message provided (use -m or resolve a merge in progress)"
                )
            })?,
        };

        {
            let index = self.index();
            let mut index = index.lock().await;
            index.rehydrate()?;

            if index.has_conflicts() {
                anyhow::bail!(
                    "Cannot commit: unresolved merge conflicts. Resolve conflicts and stage the files first."
                );
            }
        }

        let head_parent = self.refs().read_head()?;
        let merge_head = self.refs().read_merge_head()?;

        let is_root = if head_parent.is_none() {
            "(root-commit) "
        } else {
            ""
        };

        let parents: Vec<_> = head_parent.into_iter().chain(merge_head).collect();

        let commit = self.write_commit(parents, message).await?;
        let commit_id = commit.object_id()?;

        // Clear merge state after a successful merge commit
        self.refs().clear_merge_head()?;
        self.refs().clear_merge_msg()?;

        write!(
            self.writer(),
            "[{}{}] {}",
            is_root,
            &commit_id.as_ref()[..7],
            commit.short_message()
        )?;

        Ok(())
    }
}
