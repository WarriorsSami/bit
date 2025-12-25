use crate::areas::repository::Repository;
use anyhow::Context;
use std::fs;

const DEFAULT_BRANCH: &str = "master";

impl Repository {
    pub async fn init(&mut self) -> anyhow::Result<()> {
        fs::create_dir_all(self.database().objects_path())
            .context("Failed to create .git/objects directory")?;

        fs::create_dir_all(self.refs().refs_path())
            .context("Failed to create .git/refs directory")?;

        fs::create_dir_all(self.refs().heads_path())
            .context("Failed to create .git/refs/heads directory")?;

        self.refs()
            .set_head(
                DEFAULT_BRANCH,
                format!("ref: refs/heads/{}", DEFAULT_BRANCH),
            )
            .context("Failed to create initial HEAD reference")?;

        // make sure the DEFAULT_BRANCH file exists
        let head_ref_path = self.refs().heads_path().join(DEFAULT_BRANCH);
        if !head_ref_path.exists() {
            fs::write(&head_ref_path, b"").context("Failed to create default branch file")?;
        }

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
