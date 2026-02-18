use crate::areas::repository::Repository;
use crate::artifacts::objects::commit::{Author, Commit};
use crate::artifacts::objects::object::Object;
use crate::artifacts::objects::object_id::ObjectId;
use crate::artifacts::objects::tree::Tree;

impl Repository {
    pub async fn write_commit(
        &mut self,
        parents: Vec<ObjectId>,
        message: String,
    ) -> anyhow::Result<Commit> {
        let tree_id = self.write_tree().await?;

        let author = Author::load_from_env()?;
        let commit = Commit::new(parents, tree_id, author, message);
        let commit_id = commit.object_id()?;

        self.database().store(commit.clone())?;
        self.refs().update_head(commit_id)?;

        Ok(commit)
    }

    async fn write_tree(&mut self) -> anyhow::Result<ObjectId> {
        let index = self.index();
        let mut index = index.lock().await;

        // Load the index file from the disk
        index.rehydrate()?;

        let tree = Tree::build(index.entries())?;
        let tree_id = tree.object_id()?;
        let store_tree = &|tree: &Tree| self.database().store(tree.clone());
        tree.traverse(store_tree)?;

        Ok(tree_id)
    }
}
