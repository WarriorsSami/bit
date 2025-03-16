use crate::domain::areas::repository::Repository;
use crate::domain::objects::blob::Blob;
use crate::domain::objects::commit::{Author, Commit};
use crate::domain::objects::entry::Entry;
use crate::domain::objects::object::Object;
use crate::domain::objects::tree::Tree;
use anyhow::Context;
use std::io::Write;

impl Repository {
    pub fn commit(&mut self, message: &str) -> anyhow::Result<()> {
        let files = self.workspace().list_files(self.workspace().path())?;

        let entries = files
            .into_iter()
            .map(|path| {
                let data = self.workspace().read_file(&path)?;
                let blob = Blob::new(data.as_str());
                let blob_id = blob.object_id()?;

                self.database().store(blob)?;

                let stat = self.workspace().stat_file(&path)?;
                let file_name = path
                    .file_name()
                    .context("Invalid file name")?
                    .to_string_lossy()
                    .to_string();
                Ok(Entry::new(file_name, blob_id, stat))
            })
            .collect::<anyhow::Result<Vec<Entry>>>()?;

        let tree = Tree::new(entries);
        let tree_id = tree.object_id()?;
        self.database().store(tree)?;

        let parent = self.refs().read_head();
        let is_root = match parent {
            Some(_) => "",
            None => "(root-commit) ",
        };

        let author = Author::load_from_env()?;
        let message = message.trim().to_string();

        let commit = Commit::new(parent.as_deref(), tree_id.as_str(), author, message);
        let commit_id = commit.object_id()?;
        self.database().store(commit.clone())?;
        self.refs().update_head(commit_id.as_str())?;

        write!(
            self.writer(),
            "[{}{}] {}",
            is_root,
            commit_id,
            commit.short_message()
        )?;

        Ok(())
    }
}
