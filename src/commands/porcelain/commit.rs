use crate::domain::areas::repository::Repository;
use crate::domain::objects::blob::Blob;
use crate::domain::objects::commit::{Author, Commit};
use crate::domain::objects::object::Object;
use crate::domain::objects::tree::{Tree, TreeEntry};
use std::io::Write;

impl Repository {
    pub fn commit(&mut self, message: String) -> anyhow::Result<()> {
        let entries = self
            .workspace()
            .list_files()?
            .iter()
            .map(|path| {
                let data = self.workspace().read_file(path)?;
                let blob = Blob::new(data);
                let blob_id = blob.object_id()?;

                self.database().store(blob)?;

                Ok((path.into(), blob_id))
            })
            .collect::<anyhow::Result<Vec<TreeEntry>>>()?;

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

        let commit = Commit::new(parent, tree_id, author, message);
        let commit_id = commit.object_id()?;
        self.database().store(commit.clone())?;
        self.refs().update_head(commit_id.clone())?;

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
