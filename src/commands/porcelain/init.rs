use crate::domain::areas::repository::Repository;
use anyhow::Context;
use std::fs;

impl Repository {
    pub async fn init(&mut self) -> anyhow::Result<()> {
        fs::create_dir_all(self.database().objects_path())
            .context("Failed to create .git/objects directory")?;

        fs::create_dir_all(self.refs().refs_path())
            .context("Failed to create .git/refs directory")?;

        fs::write(self.refs().head_path(), "ref: refs/heads/master\n")
            .context("Failed to write .git/HEAD file")?;

        let index = self.index();
        let index = index.lock().await;
        // create the index file if it does not exist
        if !index.path().exists() {
            fs::write(index.path(), b"").context("Failed to create .git/index file")?;
        }

        write!(
            self.writer(),
            "Initialized empty Git repository in {}",
            self.path().display()
        )?;

        Ok(())
    }
}
