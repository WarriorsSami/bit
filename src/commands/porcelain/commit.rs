use crate::domain::areas::repository::Repository;
use crate::domain::objects::commit::{Author, Commit};
use crate::domain::objects::object::Object;
use crate::domain::objects::tree::Tree;
use std::io::Write;

impl Repository {
    pub async fn commit(&mut self, message: &str) -> anyhow::Result<()> {
        let index = self.index();
        let mut index = index.lock().await;

        // Load the index file from the disk
        index.rehydrate()?;

        let tree = Tree::build(index.entries())?;
        let tree_id = tree.object_id()?;
        let store_tree = &|tree: &Tree| self.database().store(tree.clone());
        tree.traverse(store_tree)?;

        let parent = self.refs().read_head()?;
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
            &commit_id.as_ref()[..7],
            commit.short_message()
        )?;

        Ok(())
    }
}
