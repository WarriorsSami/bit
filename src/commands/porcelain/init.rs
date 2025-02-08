use crate::domain::areas::repository::Repository;
use anyhow::Context;
use std::fs;

impl Repository {
    pub fn init(&mut self) -> anyhow::Result<()> {
        fs::create_dir_all(self.git_objects_path())
            .context("Failed to create .git/objects directory")?;

        fs::create_dir_all(self.git_refs_path()).context("Failed to create .git/refs directory")?;

        fs::write(self.git_head_path(), "ref: refs/heads/master\n")
            .context("Failed to write .git/HEAD file")?;

        write!(
            self.writer(),
            "Initialized git directory at {}",
            self.path().display()
        )?;

        Ok(())
    }
}
