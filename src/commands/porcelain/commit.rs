use crate::areas::repository::Repository;
use crate::artifacts::objects::object::Object;
use std::io::Write;

impl Repository {
    pub async fn commit(&mut self, message: &str) -> anyhow::Result<()> {
        let message = message.trim().to_string();

        let parent = self.refs().read_head()?;
        let is_root = match parent {
            Some(_) => "",
            None => "(root-commit) ",
        };

        let parents = parent.into_iter().collect();
        let commit = self.write_commit(parents, message).await?;
        let commit_id = commit.object_id()?;

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
