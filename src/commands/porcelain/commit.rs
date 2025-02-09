use std::io::Write;
use crate::domain::areas::repository::Repository;
use crate::domain::objects::blob::Blob;
use crate::domain::objects::commit::{Author, Commit};
use crate::domain::objects::object::Object;
use crate::domain::objects::tree::{Tree, TreeEntry};

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

        let author = Author::load_from_env()?;
        let message = message.trim().to_string();
        
        let commit = Commit::new(tree_id, author, message);
        let commit_id = commit.object_id()?;
        self.database().store(commit.clone())?;
        
        // open HEAD file as WRONLY and CREAT to write commit_id to it
        let mut head_file = std::fs::OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(self.git_head_path())?;
        head_file.write_all(commit_id.as_bytes())?;
        
        write!(self.writer(), "root-commit: {}\n{}", commit_id, commit.short_message())?;
        
        Ok(())
    }
}
