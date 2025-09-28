use crate::domain::areas::repository::Repository;
use crate::domain::objects::object_id::ObjectId;
use std::path::Path;

impl Repository {
    // TODO: add support for the recursive flag
    pub async fn ls_tree(&mut self, object_sha: &str, _recursive: bool) -> anyhow::Result<()> {
        let oid = if object_sha == "HEAD" {
            let head = self
                .refs()
                .read_head()
                .ok_or_else(|| anyhow::anyhow!("HEAD is a symbolic reference"))?;
            ObjectId::try_parse(head)?
        } else {
            ObjectId::try_parse(object_sha.to_string())?
        };

        match self.database().parse_object_as_commit(&oid)? {
            Some(commit) => {
                self.parse_tree(commit.tree_oid(), None).await?;
            }
            None => {
                self.parse_tree(&oid, None).await?;
            }
        }

        Ok(())
    }

    async fn parse_tree(&self, oid: &ObjectId, prefix: Option<&Path>) -> anyhow::Result<()> {
        let object = self.database().parse_object_as_tree(oid)?;

        match object {
            None => Ok(()),
            Some(tree) => {
                for (name, entry) in tree.into_entries() {
                    let path = if let Some(prefix) = prefix {
                        prefix.join(name)
                    } else {
                        Path::new(&name).to_path_buf()
                    };

                    if entry.is_tree() {
                        Box::pin(self.parse_tree(&entry.oid, Some(&path))).await?;
                    } else {
                        writeln!(
                            self.writer(),
                            "{:o} {} {}",
                            entry.mode.as_u32(),
                            entry.oid.as_ref(),
                            path.display()
                        )?;
                    }
                }

                Ok(())
            }
        }
    }
}
