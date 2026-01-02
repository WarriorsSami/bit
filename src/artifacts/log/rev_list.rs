use crate::areas::repository::Repository;
use crate::artifacts::branch::revision::Revision;
use crate::artifacts::objects::commit::Commit;
use crate::artifacts::objects::object_id::ObjectId;
use derive_new::new;

#[derive(Clone, new)]
pub struct RevList<'r> {
    repository: &'r Repository,
    start_revision: Revision,
}

impl<'r> RevList<'r> {
    pub fn into_iter(self) -> anyhow::Result<RevListIntoIter<'r>> {
        Ok(RevListIntoIter {
            repository: self.repository,
            current_commit_oid: self.start_revision.resolve(self.repository)?,
        })
    }
}

#[derive(Clone)]
pub struct RevListIntoIter<'r> {
    repository: &'r Repository,
    current_commit_oid: Option<ObjectId>,
}

impl Iterator for RevListIntoIter<'_> {
    type Item = Commit;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(commit_oid) = &self.current_commit_oid {
            match self
                .repository
                .database()
                .parse_object_as_commit(commit_oid)
            {
                Ok(Some(commit)) => {
                    // Move to the parent commit for the next iteration
                    self.current_commit_oid = commit.parent().cloned();
                    Some(commit)
                }
                _ => {
                    // If we can't parse the commit, end the iteration
                    self.current_commit_oid = None;
                    None
                }
            }
        } else {
            None
        }
    }
}
