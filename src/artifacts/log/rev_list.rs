use crate::areas::repository::Repository;
use crate::artifacts::branch::revision::Revision;
use crate::artifacts::objects::commit::Commit;
use crate::artifacts::objects::object_id::ObjectId;

#[derive(Clone)]
pub struct RevList<'r> {
    repository: &'r Repository,
    start_revision: Revision,
    current_commit_oid: Option<ObjectId>,
}

impl<'r> RevList<'r> {
    pub fn new(repository: &'r Repository, start_revision: Revision) -> anyhow::Result<Self> {
        let current_commit_oid = start_revision.resolve(repository)?;

        Ok(Self {
            repository,
            start_revision,
            current_commit_oid,
        })
    }

    pub fn into_iter(self) -> RevListIntoIter<'r> {
        RevListIntoIter { rev_list: self }
    }
}

#[derive(Clone)]
pub struct RevListIntoIter<'r> {
    rev_list: RevList<'r>,
}

impl Iterator for RevListIntoIter<'_> {
    type Item = Commit;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(commit_oid) = &self.rev_list.current_commit_oid {
            match self
                .rev_list
                .repository
                .database()
                .parse_object_as_commit(commit_oid)
            {
                Ok(Some(commit)) => {
                    // Move to the parent commit for the next iteration
                    self.rev_list.current_commit_oid = commit.parent().cloned();
                    Some(commit)
                }
                _ => {
                    // If we can't parse the commit, end the iteration
                    self.rev_list.current_commit_oid = None;
                    None
                }
            }
        } else {
            None
        }
    }
}
