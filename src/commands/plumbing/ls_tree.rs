use crate::domain::areas::repository::Repository;
use crate::domain::objects::database_entry::DatabaseEntry;
use crate::domain::objects::object_id::ObjectId;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

impl Repository {
    // TODO: add support for the recursive flag
    pub async fn ls_tree(&mut self, object_sha: &str, _recursive: bool) -> anyhow::Result<()> {
        let oid = if object_sha == "HEAD" {
            self.refs()
                .read_head()?
                .ok_or_else(|| anyhow::anyhow!("HEAD is not a symbolic reference"))?
        } else {
            ObjectId::try_parse(object_sha.to_string())?
        };

        match self.database().parse_object_as_commit(&oid)? {
            Some(commit) => {
                self.parse_tree(commit.tree_oid(), None, &mut Default::default(), true)
                    .await?;
            }
            None => {
                self.parse_tree(&oid, None, &mut Default::default(), true)
                    .await?;
            }
        }

        Ok(())
    }

    pub(crate) async fn parse_tree(
        &self,
        oid: &ObjectId,
        prefix: Option<&Path>,
        tree_data: &mut BTreeMap<PathBuf, DatabaseEntry>,
        should_print: bool,
    ) -> anyhow::Result<()> {
        if let Some(tree) = self.database().parse_object_as_tree(oid)? {
            for (name, entry) in tree.into_entries() {
                let path = if let Some(prefix) = prefix {
                    prefix.join(name)
                } else {
                    Path::new(&name).to_path_buf()
                };

                if entry.is_tree() {
                    Box::pin(self.parse_tree(&entry.oid, Some(&path), tree_data, should_print))
                        .await?;
                } else {
                    if should_print {
                        writeln!(
                            self.writer(),
                            "{:o} {} {}",
                            entry.mode.as_u32(),
                            entry.oid.as_ref(),
                            path.display()
                        )?;
                    }
                    tree_data.insert(path, entry);
                }
            }
        }

        Ok(())
    }
}
