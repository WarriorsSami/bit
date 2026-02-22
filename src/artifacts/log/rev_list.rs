//! Revision list traversal for git log
//!
//! This module implements the core algorithm for traversing commit history,
//! similar to `git rev-list`. It supports:
//!
//! - Topological ordering by timestamp
//! - Range expressions (commit1..commit2)
//! - Excluded revisions (^commit)
//! - Path filtering (show only commits affecting specific files)
//! - Handling of merge commits and complex histories
//!
//! ## Algorithm
//!
//! The traversal uses a priority queue ordered by commit timestamp to process
//! commits in reverse chronological order. Commits are marked with flags to track
//! their state (seen, added, uninteresting, etc.) and ensure correct handling
//! of complex revision graphs.
//!
//! ## Edge Cases
//!
//! Special handling for:
//! - Commits with identical timestamps (uses OID as tiebreaker)
//! - Uninteresting commits that might still lead to interesting ones
//! - Path filtering that requires tree diffing

use crate::areas::refs::HEAD_REF_NAME;
use crate::areas::repository::Repository;
use crate::artifacts::branch::revision::Revision;
use crate::artifacts::diff::tree_diff::TreeDiff;
use crate::artifacts::log::path_filter::PathFilter;
use crate::artifacts::objects::commit::Commit;
use crate::artifacts::objects::object::Object;
use crate::artifacts::objects::object_id::ObjectId;
use crate::commands::porcelain::log::LogRevisionTargets;
use std::cmp::Ordering;
use std::collections::{BinaryHeap, HashMap, HashSet, VecDeque};
use std::path::PathBuf;

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
    /// The commit has the same tree as its parent (used for filtering).
    TreeSame,
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

/// Type alias for commit diffs cache
pub type CommitsDiffs<'r> = HashMap<(Option<ObjectId>, Option<ObjectId>), TreeDiff<'r>>;

/// Revision list traverser
///
/// Implements the core git log algorithm for traversing commit history
/// with support for range expressions, excluded revisions, and path filtering.
///
/// ## Fields
///
/// - `commits_cache`: Cached commit objects to avoid redundant database reads
/// - `commits_flags`: Tracking flags for each commit's processing state
/// - `commits_pqueue`: Priority queue ordered by timestamp for chronological traversal
/// - `commits_output_list`: Final ordered list of commits to display
/// - `path_filter`: Optional filter to show only commits affecting specific paths
/// - `commits_diffs`: Cached tree diffs for path filtering
pub struct RevList<'r> {
    repository: &'r Repository,
    /// Cache of loaded commit objects
    commits_cache: HashMap<ObjectId, Commit>,
    /// Processing state flags for each commit
    commits_flags: HashMap<ObjectId, HashSet<LogTraversalCommitFlag>>,
    /// Priority queue for timestamp-ordered traversal
    commits_pqueue: BinaryHeap<CommitQueueEntry>,
    /// Whether the traversal uses exclusion/range expressions
    is_limited: bool,
    /// Final output list of commits
    commits_output_list: Vec<Commit>,
    /// Paths to filter by
    interesting_files: Vec<PathBuf>,
    /// Cached tree diffs for path filtering
    commits_diffs: CommitsDiffs<'r>,
    /// Trie-based path filter
    path_filter: PathFilter,
}

impl<'r> RevList<'r> {
    pub fn new(
        repository: &'r Repository,
        mut target_revisions: Vec<LogRevisionTargets>,
        target_files: Option<Vec<PathBuf>>,
    ) -> anyhow::Result<Self> {
        let mut rev_list = Self {
            repository,
            commits_cache: HashMap::new(),
            commits_flags: HashMap::new(),
            commits_pqueue: BinaryHeap::new(),
            is_limited: false,
            commits_output_list: Vec::new(),
            interesting_files: Vec::new(),
            commits_diffs: HashMap::new(),
            path_filter: PathFilter::empty(),
        };

        let interesting_files = if let Some(files) = target_files {
            let mut interesting_files = vec![];
            for file in files {
                let _file_stat = rev_list.repository.workspace().stat_file(file.as_ref())?;

                interesting_files.push(file);
            }

            rev_list.is_limited = true;

            interesting_files
        } else {
            vec![]
        };

        rev_list.interesting_files = interesting_files.clone();
        rev_list.path_filter = PathFilter::new(interesting_files);

        // Edge case: no targets provided excepting excluded revisions
        if target_revisions
            .iter()
            .all(|t| matches!(t, LogRevisionTargets::ExcludedRevision(_)))
        {
            // Default to including HEAD
            target_revisions.extend(vec![LogRevisionTargets::IncludedRevision(
                Revision::try_parse(HEAD_REF_NAME)?,
            )]);
        }

        // Initialize the priority queue with all starting commits from targets
        for target_revision in target_revisions {
            rev_list.handle_target_revision(target_revision)?;
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

    pub fn commit_diffs(&self) -> &CommitsDiffs<'r> {
        &self.commits_diffs
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
                        && !flags.contains(&LogTraversalCommitFlag::TreeSame)
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

            // Edge case: Compare the oldest interesting commit in output with the newest commit in the queue
            // to decide whether we could still reach interesting commits from uninteresting ones,
            // thus marking also the formers as uninteresting.
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

    fn handle_target_revision(&mut self, target: LogRevisionTargets) -> anyhow::Result<()> {
        match target {
            LogRevisionTargets::IncludedRevision(revision) => {
                self.handle_start_revision(revision, true)?;
            }
            LogRevisionTargets::RangeExpression { excluded, included } => {
                self.handle_start_revision(excluded, false)?;
                self.handle_start_revision(included, true)?;
            }
            LogRevisionTargets::ExcludedRevision(revision) => {
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
        let commit = if let Some(commit) = self.commits_cache.get(oid) {
            commit
        } else {
            return Ok(());
        };

        let mut commits_queue = commit.parents().iter().collect::<VecDeque<_>>();

        while let Some(oid) = commits_queue.pop_front() {
            if !self
                .commits_flags
                .entry(oid.clone())
                .or_default()
                .insert(LogTraversalCommitFlag::Uninteresting)
            {
                continue;
            }

            if let Some(commit) = self.commits_cache.get(oid) {
                commits_queue.extend(commit.parents());
            }
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

        let is_uninteresting = self
            .commits_flags
            .get(oid)
            .is_some_and(|flags| flags.contains(&LogTraversalCommitFlag::Uninteresting));

        let parents = if is_uninteresting {
            commit
                .parents()
                .iter()
                .map(|parent_oid| {
                    self.load_commit(parent_oid)?;
                    self.mark_parents_uninteresting(oid)?;

                    Ok(parent_oid)
                })
                .collect::<anyhow::Result<Vec<_>>>()?
        } else {
            self.simplify_commit(&commit)?
                .into_iter()
                .flatten()
                .map(|parent_oid| {
                    self.load_commit(parent_oid)?;

                    Ok(parent_oid)
                })
                .collect::<anyhow::Result<Vec<_>>>()?
        };

        parents
            .into_iter()
            .try_for_each(|parent_oid| self.enqueue_commit(parent_oid))?;

        Ok(())
    }

    fn simplify_commit<'a>(
        &mut self,
        commit: &'a Commit,
    ) -> anyhow::Result<Vec<Option<&'a ObjectId>>> {
        let parents = if commit.parents().is_empty() {
            vec![None]
        } else {
            commit.parents().iter().map(Some).collect()
        };

        if self.interesting_files.is_empty() {
            return Ok(parents);
        }

        let commit_oid = commit.object_id()?;

        for parent in parents.iter() {
            if self
                .tree_diff(*parent, Some(&commit_oid))?
                .changes()
                .is_empty()
            {
                self.commits_flags
                    .entry(commit_oid.clone())
                    .or_default()
                    .insert(LogTraversalCommitFlag::TreeSame);

                return Ok(vec![*parent]);
            }
        }

        Ok(parents)
    }

    fn tree_diff(
        &mut self,
        old: Option<&ObjectId>,
        new: Option<&ObjectId>,
    ) -> anyhow::Result<TreeDiff<'r>> {
        let key = (old.cloned(), new.cloned());
        if let Some(tree_diff) = self.commits_diffs.get(&key) {
            return Ok(tree_diff.clone());
        }

        let tree_diff = self
            .repository
            .database()
            .tree_diff(old, new, &self.path_filter)?;
        self.commits_diffs.insert(key, tree_diff.clone());

        Ok(tree_diff)
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
