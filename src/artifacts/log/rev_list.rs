use crate::areas::refs::HEAD_REF_NAME;
use crate::areas::repository::Repository;
use crate::artifacts::branch::revision::Revision;
use crate::artifacts::objects::commit::Commit;
use crate::artifacts::objects::object::Object;
use crate::artifacts::objects::object_id::ObjectId;
use crate::commands::porcelain::log::LogTarget;
use std::cmp::Ordering;
use std::collections::{BinaryHeap, HashMap, HashSet};

/// A flag indicating the state of a commit during log traversal.
///
/// This enum tracks the processing state of commits as they move through
/// the log traversal algorithm, ensuring each commit is processed exactly once.
#[derive(Debug, Clone, Hash, PartialEq, Eq, Default)]
enum LogTraversalCommitFlag {
    /// The commit has been fetched from the database.
    #[default]
    Fetched,
    /// The commit has been seen during traversal and is in the priority queue.
    Seen,
    /// The commit has been added to the final output list.
    Added,
    /// The commit is uninteresting and should be skipped as it's reachable from an excluded revision.
    Uninteresting,
}

/// Wrapper for ObjectId in the priority queue that orders by commit timestamp.
/// This allows the priority queue to work with ObjectIds while maintaining
/// timestamp-based ordering by looking up commits in the cache.
#[derive(Debug, Clone, Eq, PartialEq)]
struct CommitQueueEntry {
    oid: ObjectId,
    timestamp: chrono::DateTime<chrono::FixedOffset>,
}

impl TryFrom<&Commit> for CommitQueueEntry {
    type Error = anyhow::Error;

    fn try_from(commit: &Commit) -> Result<Self, Self::Error> {
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
    commits_cache: HashMap<ObjectId, Commit>,
    commits_flags: HashMap<ObjectId, HashSet<LogTraversalCommitFlag>>,
    commits_pqueue: BinaryHeap<CommitQueueEntry>,
    is_limited: bool,
    commits_output_list: Vec<Commit>,
}

impl<'r> RevList<'r> {
    pub fn new(repository: &'r Repository, targets: Vec<LogTarget>) -> anyhow::Result<Self> {
        let mut rev_list = Self {
            repository,
            commits_cache: HashMap::new(),
            commits_flags: HashMap::new(),
            commits_pqueue: BinaryHeap::new(),
            is_limited: false,
            commits_output_list: Vec::new(),
        };

        // Edge case: no targets provided excepting excluded revisions
        let targets = if targets
            .iter()
            .all(|t| matches!(t, LogTarget::ExcludedRevision(_)))
        {
            targets
                .into_iter()
                .chain(std::iter::once(LogTarget::IncludedRevision(
                    Revision::try_parse(HEAD_REF_NAME)?,
                )))
                .collect()
        } else {
            targets
        };

        // Initialize the priority queue with all starting commits from targets
        for target in targets {
            rev_list.handle_log_target(target)?;
        }

        // Scan the history graph and build the output list only including interesting commits
        if rev_list.is_limited {
            rev_list.limit_to_interesting_commits()?;
        }

        Ok(rev_list)
    }

    pub fn into_iter(self) -> RevListIntoIter<'r> {
        RevListIntoIter { rev_list: self }
    }

    fn limit_to_interesting_commits(&mut self) -> anyhow::Result<()> {
        while self.is_still_interesting()?
            && let Some(entry) = self.commits_pqueue.pop()
        {
            let commit = if let Some(commit) = self.commits_cache.get(&entry.oid) {
                commit.clone()
            } else {
                continue; // Commit not found in cache, skip
            };

            // Add parents to the queue
            self.add_parent(&entry.oid)?;

            // Only add interesting commits to the output list
            if let Some(flags) = self.commits_flags.get(&entry.oid)
                && !flags.contains(&LogTraversalCommitFlag::Uninteresting)
            {
                self.commits_output_list.push(commit);
            }
        }

        // Rebuild the priority queue from the output list for final iteration,
        // this time including only interesting commits
        self.commits_pqueue =
            self.commits_output_list
                .iter()
                .fold(BinaryHeap::new(), |mut heap, commit| {
                    if let Ok(entry) = CommitQueueEntry::try_from(commit)
                        && let Some(flags) = self.commits_flags.get(&entry.oid)
                        && !flags.contains(&LogTraversalCommitFlag::Uninteresting)
                    {
                        heap.push(entry);
                    }
                    heap
                });

        Ok(())
    }

    fn is_still_interesting(&self) -> anyhow::Result<bool> {
        if let Some(newest_in_queue) = self.commits_pqueue.peek().cloned() {
            let oldest_in_output = self
                .commits_output_list
                .last()
                .map(CommitQueueEntry::try_from)
                .transpose()?;

            // Compare the oldest interesting commit in output with the newest commit in the queue
            // to decide whether we could still reach interesting commits from uninteresting ones
            if let Some(oldest_in_output) = oldest_in_output
                && oldest_in_output.timestamp <= newest_in_queue.timestamp
            {
                Ok(true)
            } else {
                // Default to checking whether there are any interesting commits left in the queue
                let is_any_interesting_commit_left = self.commits_pqueue.iter().any(|entry| {
                    self.commits_flags.get(&entry.oid).is_some_and(|flags| {
                        !flags.contains(&LogTraversalCommitFlag::Uninteresting)
                    })
                });
                Ok(is_any_interesting_commit_left)
            }
        } else {
            Ok(false)
        }
    }

    fn handle_log_target(&mut self, target: LogTarget) -> anyhow::Result<()> {
        match target {
            LogTarget::IncludedRevision(revision) => {
                self.handle_start_revision(revision, true)?;
            }
            LogTarget::RangeExpression { excluded, included } => {
                self.handle_start_revision(excluded, false)?;
                self.handle_start_revision(included, true)?;
            }
            LogTarget::ExcludedRevision(revision) => {
                self.handle_start_revision(revision, false)?;
            }
        }

        Ok(())
    }

    fn handle_start_revision(
        &mut self,
        revision: Revision,
        is_interesting: bool,
    ) -> anyhow::Result<()> {
        let commit_oid = revision
            .resolve(self.repository)?
            .ok_or_else(|| anyhow::anyhow!("Failed to resolve revision"))?;

        self.load_commit(&commit_oid)?;
        self.enqueue_commit(&commit_oid)?;

        if !is_interesting {
            self.is_limited = true;

            self.commits_flags
                .entry(commit_oid.clone())
                .or_default()
                .insert(LogTraversalCommitFlag::Uninteresting);
            self.mark_parents_uninteresting(&commit_oid)?;
        }

        Ok(())
    }

    fn mark_parents_uninteresting(&mut self, oid: &ObjectId) -> anyhow::Result<()> {
        let mut commit = if let Some(commit) = self.commits_cache.get(oid) {
            commit
        } else {
            return Ok(());
        };

        while let Some(parent_oid) = commit.parent()
            && let Some(parent_commit) = self.commits_cache.get(parent_oid)
        {
            if !self
                .commits_flags
                .entry(parent_oid.clone())
                .or_default()
                .insert(LogTraversalCommitFlag::Uninteresting)
            {
                break;
            }

            commit = parent_commit;
        }

        Ok(())
    }

    fn load_commit(&mut self, oid: &ObjectId) -> anyhow::Result<Option<&Commit>> {
        if !self.commits_cache.contains_key(oid) {
            if let Some(commit) = self.repository.database().parse_object_as_commit(oid)? {
                self.commits_cache.insert(oid.clone(), commit);
                self.commits_flags
                    .entry(oid.clone())
                    .or_default()
                    .insert(LogTraversalCommitFlag::Fetched);
            } else {
                anyhow::bail!("Commit {} not found in repository database", oid);
            }
        }

        Ok(self.commits_cache.get(oid))
    }

    fn enqueue_commit(&mut self, oid: &ObjectId) -> anyhow::Result<()> {
        // Get the commit and check its current flag
        if let Some(commit) = self.commits_cache.get(oid)
            && let Some(flags) = self.commits_flags.get_mut(oid)
            && !flags.contains(&LogTraversalCommitFlag::Seen)
        {
            flags.insert(LogTraversalCommitFlag::Seen);

            // Add to the priority queue
            self.commits_pqueue.push(commit.try_into()?);
        }

        Ok(())
    }

    fn add_parent(&mut self, oid: &ObjectId) -> anyhow::Result<()> {
        let commit = if let Some(commit) = self.commits_cache.get(oid)
            && let Some(flags) = self.commits_flags.get_mut(oid)
            && !flags.contains(&LogTraversalCommitFlag::Added)
        {
            flags.insert(LogTraversalCommitFlag::Added);
            commit.clone()
        } else {
            return Ok(());
        };

        // Load its parent into the cache if not already present and enqueue the parent if found
        if let Some(parent_oid) = commit.parent() {
            self.load_commit(parent_oid)?;

            // Mark parents as uninteresting if this commit is uninteresting
            if let Some(flags) = self.commits_flags.get(oid)
                && flags.contains(&LogTraversalCommitFlag::Uninteresting)
            {
                self.mark_parents_uninteresting(oid)?;
            }

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
            let commit = if let Some(commit) = self.rev_list.commits_cache.get(&entry.oid) {
                commit.clone()
            } else {
                return None;
            };

            // Avoid adding parents again if the rev_list is limited,
            // as they were already traversed during limiting
            if !self.rev_list.is_limited
                && let Err(err) = self.rev_list.add_parent(&entry.oid)
            {
                eprintln!("Error adding parent commit: {}", err);
                return None;
            }

            Some(commit)
        })
    }
}
