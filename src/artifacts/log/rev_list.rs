use crate::areas::repository::Repository;
use crate::artifacts::objects::commit::Commit;
use crate::artifacts::objects::object::Object;
use crate::artifacts::objects::object_id::ObjectId;
use crate::commands::porcelain::log::LogTarget;
use std::cmp::Ordering;
use std::collections::{BinaryHeap, HashMap};

/// A flag indicating the state of a commit during log traversal.
///
/// This enum tracks the processing state of commits as they move through
/// the log traversal algorithm, ensuring each commit is processed exactly once.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
enum LogTraversalCommitFlag {
    /// The commit has been fetched from the database.
    #[default]
    Fetched,
    /// The commit has been seen during traversal and is in the priority queue.
    Seen,
    /// The commit has been added to the final output list.
    Added,
}

type LogCommitRecord = (Commit, LogTraversalCommitFlag);

/// Wrapper for ObjectId in the priority queue that orders by commit timestamp.
/// This allows the priority queue to work with ObjectIds while maintaining
/// timestamp-based ordering by looking up commits in the cache.
#[derive(Debug, Clone, Eq, PartialEq)]
struct CommitQueueEntry {
    oid: ObjectId,
    timestamp: chrono::DateTime<chrono::FixedOffset>,
}

impl TryFrom<&mut Commit> for CommitQueueEntry {
    type Error = anyhow::Error;

    fn try_from(commit: &mut Commit) -> Result<Self, Self::Error> {
        Ok(CommitQueueEntry {
            oid: commit.object_id()?,
            timestamp: commit.timestamp(),
        })
    }
}

impl Ord for CommitQueueEntry {
    fn cmp(&self, other: &Self) -> Ordering {
        // Newer timestamps come first (reverse order for max-heap)
        self.timestamp
            .cmp(&other.timestamp)
            // Use OID as tiebreaker for stable ordering
            .then_with(|| self.oid.cmp(&other.oid))
    }
}

impl PartialOrd for CommitQueueEntry {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

pub struct RevList<'r> {
    repository: &'r Repository,
    commits_cache: HashMap<ObjectId, LogCommitRecord>,
    commits_pqueue: BinaryHeap<CommitQueueEntry>,
}

impl<'r> RevList<'r> {
    pub fn new(repository: &'r Repository, targets: Vec<LogTarget>) -> anyhow::Result<Self> {
        let mut rev_list = Self {
            repository,
            commits_cache: HashMap::new(),
            commits_pqueue: BinaryHeap::new(),
        };

        // Initialize the priority queue with all starting commits from targets
        for target in targets {
            rev_list.handle_log_target(target)?;
        }

        Ok(rev_list)
    }

    pub fn into_iter(self) -> RevListIntoIter<'r> {
        RevListIntoIter { rev_list: self }
    }

    fn load_commit(&mut self, oid: &ObjectId) -> anyhow::Result<Option<&Commit>> {
        if !self.commits_cache.contains_key(oid) {
            if let Some(commit) = self.repository.database().parse_object_as_commit(oid)? {
                self.commits_cache
                    .insert(oid.clone(), (commit, Default::default()));
            } else {
                anyhow::bail!("Commit {} not found in repository database", oid);
            }
        }

        Ok(self.commits_cache.get(oid).map(|(commit, _)| commit))
    }

    fn handle_log_target(&mut self, target: LogTarget) -> anyhow::Result<()> {
        match target {
            LogTarget::IncludedRevision(revision) => {
                let commit_oid = revision
                    .resolve(self.repository)?
                    .ok_or_else(|| anyhow::anyhow!("Failed to resolve revision"))?;

                self.load_commit(&commit_oid)?;
                self.enqueue_commit(&commit_oid)?;
            }
            LogTarget::RangeExpression {
                excluded: _excluded,
                included: _included,
            } => {
                todo!()
            }
            LogTarget::ExcludedRevision(_revision) => {
                todo!()
            }
        }

        Ok(())
    }

    fn enqueue_commit(&mut self, oid: &ObjectId) -> anyhow::Result<()> {
        // Get the commit and check its current flag
        if let Some((commit, flag)) = self.commits_cache.get_mut(oid)
            && *flag != LogTraversalCommitFlag::Seen
        {
            *flag = LogTraversalCommitFlag::Seen;

            // Add to the priority queue
            self.commits_pqueue.push(commit.try_into()?);
        }

        Ok(())
    }

    fn add_parent(&mut self, oid: &ObjectId) -> anyhow::Result<()> {
        let commit = if let Some((commit, flag)) = self.commits_cache.get_mut(oid)
            && *flag != LogTraversalCommitFlag::Added
        {
            *flag = LogTraversalCommitFlag::Added;
            commit.clone()
        } else {
            return Ok(());
        };

        // Load its parent into the cache if not already present and enqueue the parent if found
        if let Some(parent_oid) = commit.parent() {
            self.load_commit(parent_oid)?;
            self.enqueue_commit(parent_oid)?;
        }

        Ok(())
    }
}

pub struct RevListIntoIter<'r> {
    rev_list: RevList<'r>,
}

impl Iterator for RevListIntoIter<'_> {
    type Item = Commit;

    fn next(&mut self) -> Option<Self::Item> {
        self.rev_list.commits_pqueue.pop().and_then(|entry| {
            let commit = if let Some((commit, _flag)) = self.rev_list.commits_cache.get(&entry.oid)
            {
                commit.clone()
            } else {
                return None;
            };

            if let Err(err) = self.rev_list.add_parent(&entry.oid) {
                eprintln!("Error adding parent commit: {}", err);
                return None;
            }

            Some(commit)
        })
    }
}
