//! Common ancestor finder for Git merge operations
//!
//! This module implements algorithms to find the best common ancestor(s) between two commits
//! in a Git repository. The best common ancestor finder is a crucial component for three-way merges,
//! as it determines the base commit from which to calculate merge differences.
//!
//! ## Algorithm Overview
//!
//! The implementation uses a two-phase algorithm:
//!
//! ### Phase 1: Find All Common Ancestors
//!
//! A bidirectional graph traversal explores the commit history of both input commits:
//! - Process commits in reverse chronological order (oldest first)
//! - Mark commits as visited from source or target side
//! - When a commit is visited from both sides, mark it as a common ancestor
//! - Mark descendants of common ancestors as STALE to prune the search space
//!
//! ### Phase 2: Filter to Best Common Ancestors
//!
//! Apply the **Best Common Ancestor (BCA) Invariant**:
//! > A best common ancestor of commits X and Y is any common ancestor of X and Y
//! > that is not an ancestor of any other common ancestor.
//!
//! The algorithm filters redundant ancestors by:
//! - For each pair of common ancestors, check if one is reachable from the other
//! - Remove any common ancestor that is an ancestor of another common ancestor
//! - Return one of the remaining best common ancestors
//!
//! ## Supported Scenarios
//!
//! The implementation handles complex scenarios including:
//!
//! - Linear histories (ancestor-descendant relationships)
//! - Simple two-way merges
//! - Complex multi-way merges and octopus merges
//! - Criss-cross merges with multiple common ancestors
//! - Diamond-shaped merge patterns
//! - Long parallel development branches
//! - Looping histories where multiple best common ancestors exist
//!
//! ## Usage
//!
//! ```rust,ignore
//! let finder = BCAFinder::new(|commit_id| {
//!     // Your function to load commit parents and timestamp
//!     repository.get_slim_commit(commit_id)
//! });
//!
//! let best_common_ancestor = finder.find_best_common_ancestor(&commit1, &commit2);
//! ```
//!
//! ## Debug Logging
//!
//! The module includes detailed debug logging to help understand the algorithm's execution.
//! Debug logging is automatically enabled when:
//! - Running tests (`cargo test`)
//! - Using the `debug_merge` feature flag (`cargo build --features debug_merge`)
//!
//! The debug output includes:
//! - Commit processing order and visit states
//! - Final ancestor states after traversal
//! - Common ancestors found
//! - Redundant ancestor filtering
//! - Best common ancestor results
//!
//! To enable debug output in production code:
//! ```toml
//! # In Cargo.toml
//! [features]
//! debug_merge = []
//! ```
//!
//! Then build with: `cargo build --features debug_merge`
//!
//! ## Performance Considerations
//!
//! The current implementation may perform redundant work when there are many common ancestors.
//! Future optimizations could:
//! - Reduce cloning in the redundancy filter phase
//! - Use more efficient data structures for ancestor checking
//! - Implement early termination heuristics

use crate::artifacts::objects::commit::SlimCommit;
use crate::artifacts::objects::object_id::ObjectId;
use bitflags::bitflags;
use std::collections::HashMap;
use std::collections::HashSet;
use std::fmt;

/// Macro for debug logging that is enabled in test mode or with the debug_merge feature flag
///
/// # Usage
/// ```rust,ignore
/// debug_log!("Processing commit {}", commit_id);
/// ```
macro_rules! debug_log {
    ($($arg:tt)*) => {
        #[cfg(any(feature = "debug_merge"))]
        {
            eprintln!($($arg)*);
        }
    };
}

bitflags! {
    #[derive(Clone, Copy, PartialEq, Eq, Hash)]
    struct VisitState: u8 {
        const NONE = 0b00;
        const VISITED_FROM_SOURCE = 0b01;
        const VISITED_FROM_TARGET = 0b10;
        const VISITED_FROM_BOTH = Self::VISITED_FROM_SOURCE.bits() | Self::VISITED_FROM_TARGET.bits();
        const STALE = 0b100; // Optional flag to mark commits that have been fully processed
        const RESULT = 0b1000; // Optional flag to mark commits that are identified as common ancestors
    }
}

impl fmt::Debug for VisitState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut flags = Vec::new();
        if self.contains(VisitState::VISITED_FROM_SOURCE) {
            flags.push("SOURCE");
        }
        if self.contains(VisitState::VISITED_FROM_TARGET) {
            flags.push("TARGET");
        }
        if self.contains(VisitState::STALE) {
            flags.push("STALE");
        }
        if self.contains(VisitState::RESULT) {
            flags.push("RESULT");
        }
        if flags.is_empty() {
            write!(f, "NONE")
        } else {
            write!(f, "{}", flags.join("|"))
        }
    }
}

impl fmt::Display for VisitState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(self, f)
    }
}

/// Finds common ancestors between commits in a Git repository
///
/// This struct encapsulates the algorithm for finding the lowest common ancestor
/// between two commits. It takes a generic function that can load SlimCommit data
/// for any given commit, making it flexible enough to work with different
/// storage backends (file system, in-memory, database, etc.).
///
/// # Type Parameters
///
/// * `CommitLoaderFn` - A function that takes an ObjectId reference and returns
///   a SlimCommit that borrows from a commit cache. The HRTB (Higher-Rank Trait Bound)
///   `for<'c>` ensures the function works for any lifetime 'c.
#[derive(Debug, Clone)]
struct CommonAncestorsFinder<'c, CommitLoaderFn>
where
    CommitLoaderFn: Fn(&ObjectId) -> SlimCommit<'c>,
{
    /// Function to load commit data for any given commit ID
    commit_loader: CommitLoaderFn,
    _marker: std::marker::PhantomData<&'c ()>, // Marker to tie the lifetime 'c to the struct
}

impl<'c, CommitLoaderFn> CommonAncestorsFinder<'c, CommitLoaderFn>
where
    CommitLoaderFn: Fn(&ObjectId) -> SlimCommit<'c>,
{
    /// Creates a new common ancestor finder with the given commit loader function
    ///
    /// # Arguments
    ///
    /// * `commit_loader` - Function that takes a commit ObjectId and returns
    ///   a SlimCommit containing parent ObjectIds and timestamp. Should return
    ///   a SlimCommit with empty parents vector for root commits.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let finder = BCAFinder::new(|commit_id| {
    ///     database.get_slim_commit(commit_id).unwrap_or_default()
    /// });
    /// ```
    fn new(commit_loader: CommitLoaderFn) -> Self {
        Self {
            commit_loader,
            _marker: std::marker::PhantomData,
        }
    }

    /// Finds all common ancestors between a source commit and a set of target commits
    ///
    /// This is an internal method that implements a bidirectional graph traversal algorithm
    /// to find all common ancestors shared by the source and target commits. The algorithm
    /// handles various complex merge scenarios including criss-cross merges and multiple merge bases.
    ///
    /// # Arguments
    ///
    /// * `source_commit_id` - The source commit to find common ancestors for
    /// * `target_commit_ids` - The set of target commits to compare against
    ///
    /// # Returns
    ///
    /// A HashMap of commit IDs to their visit states, filtered to exclude stale commits.
    /// Common ancestors are those that have been visited from both source and target sides.
    ///
    /// # Algorithm
    ///
    /// The algorithm uses a timestamp-ordered traversal approach:
    /// 1. Start from both source and target commits simultaneously
    /// 2. Process commits in reverse chronological order (oldest first using priority queue)
    /// 3. Mark commits as visited from source or target side
    /// 4. When a commit is visited from both sides, mark it as a common ancestor (RESULT flag)
    /// 5. Mark descendants of common ancestors as STALE to avoid redundant processing
    /// 6. Continue until all reachable commits are processed
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// // Linear history: A <- B <- C <- D
    /// let ancestors = finder.find_common_ancestors(&b, HashSet::from([&d]));
    /// // Returns commits marked as common ancestors (B in this case)
    ///
    /// // Branched history:
    /// //     A
    /// //    / \
    /// //   B   C
    /// let ancestors = finder.find_common_ancestors(&b, HashSet::from([&c]));
    /// // Returns commits marked as common ancestors (A in this case)
    /// ```
    fn find_common_ancestors(
        &self,
        source_commit_id: &ObjectId,
        target_commit_ids: HashSet<&ObjectId>,
    ) -> HashMap<ObjectId, VisitState> {
        if target_commit_ids.contains(source_commit_id) {
            // If the source commit is also a target, it's the common ancestor
            return HashMap::from([(source_commit_id.clone(), VisitState::RESULT)]);
        }

        let mut ancestors_states = HashMap::<ObjectId, VisitState>::new();
        let mut priority_queue = std::collections::BinaryHeap::new();

        // Load initial commits and add to queue
        let source_commit = (self.commit_loader)(source_commit_id);

        // Add source and target commits with their respective visit states
        // Process newest commits first (max heap with timestamp)
        ancestors_states.insert(source_commit.oid.clone(), VisitState::VISITED_FROM_SOURCE);
        priority_queue.push((source_commit.timestamp, source_commit.oid.clone()));

        for &target_commit_id in target_commit_ids.iter() {
            ancestors_states.insert(target_commit_id.clone(), VisitState::VISITED_FROM_TARGET);

            let target_commit = (self.commit_loader)(target_commit_id);
            priority_queue.push((target_commit.timestamp, target_commit.oid.clone()));
        }

        while let Some((_, commit_id)) = priority_queue.pop() {
            let current_state = ancestors_states
                .get(&commit_id)
                .copied()
                .unwrap_or(VisitState::NONE);

            debug_log!("Processing commit {}: state={}", &commit_id, current_state);

            if current_state.contains(VisitState::STALE) {
                continue; // Skip already processed commits
            }

            // Check if this commit has been visited from both sides
            let is_common_ancestor = if current_state.contains(VisitState::VISITED_FROM_BOTH) {
                ancestors_states
                    .entry(commit_id.clone())
                    .and_modify(|state| *state |= VisitState::RESULT);
                true
            } else {
                false
            };

            // Load the commit to get its parents
            let current_commit = (self.commit_loader)(&commit_id);

            // Process all parents
            for parent_id in current_commit.parents {
                let parent_commit = (self.commit_loader)(parent_id);
                let parent_state = ancestors_states
                    .get(parent_id)
                    .copied()
                    .unwrap_or(VisitState::NONE);

                // Inherit visit state from current commit
                let mut new_state = parent_state | current_state;
                if is_common_ancestor {
                    new_state |= VisitState::STALE;
                }

                // Only add to queue if we haven't processed this parent with this state
                if !parent_state.contains(current_state) {
                    ancestors_states.insert(parent_id.clone(), new_state);
                    priority_queue.push((parent_commit.timestamp, parent_id.clone()));
                }
            }
        }

        debug_log!(
            "Final ancestors states: {}",
            ancestors_states
                .iter()
                .map(|(oid, state)| format!("{}: {}", oid, state))
                .collect::<Vec<_>>()
                .join(", ")
        );

        ancestors_states
            .into_iter()
            .filter(|(_, state)| {
                !state.contains(VisitState::STALE) && state.contains(VisitState::RESULT)
            })
            .collect()
    }
}

// Best Common Ancestor Finder with proper lifetime management
pub struct BCAFinder<'c, CommitLoaderFn>
where
    CommitLoaderFn: Fn(&ObjectId) -> SlimCommit<'c>,
{
    inner: CommonAncestorsFinder<'c, CommitLoaderFn>,
}

impl<'c, CommitLoaderFn> BCAFinder<'c, CommitLoaderFn>
where
    CommitLoaderFn: Fn(&ObjectId) -> SlimCommit<'c>,
{
    /// Creates a new best common ancestor finder with the given commit loader function
    ///
    /// # Arguments
    ///
    /// * `commit_loader` - Function that takes a commit ObjectId and returns
    ///   a SlimCommit containing parent ObjectIds and timestamp. Should return
    ///   a SlimCommit with empty parents vector for root commits.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let finder = BCAFinder::new(|commit_id| {
    ///     database.get_slim_commit(commit_id).unwrap_or_default()
    /// });
    /// ```
    pub fn new(commit_loader: CommitLoaderFn) -> Self {
        Self {
            inner: CommonAncestorsFinder::new(commit_loader),
        }
    }

    /// Finds the best common ancestor between two commits
    ///
    /// This method implements the best common ancestor (BCA) invariant:
    /// **A best common ancestor of commits X and Y is any common ancestor of X and Y
    /// that is not an ancestor of any other common ancestor.**
    ///
    /// The algorithm works in two phases:
    /// 1. Find all common ancestors using bidirectional graph traversal
    /// 2. Filter out redundant ancestors by checking if any common ancestor is an ancestor of another
    ///
    /// # Arguments
    ///
    /// * `source_commit_id` - The first commit to find the best common ancestor for
    /// * `target_commit_id` - The second commit to find the best common ancestor for
    ///
    /// # Returns
    ///
    /// An `Option<ObjectId>` containing:
    /// - `Some(ObjectId)` - One of the best common ancestors (if multiple exist, one is chosen)
    /// - `None` - If no common ancestor exists (e.g., commits from different repository roots)
    ///
    /// # Algorithm Details
    ///
    /// For each pair of common ancestors (A, B), the algorithm:
    /// - Finds common ancestors between A and other common ancestors
    /// - If A is reachable from any other common ancestor, A is redundant
    /// - If any other common ancestor is reachable from A, that ancestor is redundant
    /// - Filters out all redundant ancestors to find the best common ancestor(s)
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// // Linear history: A <- B <- C <- D
    /// let bca = finder.find_best_common_ancestor(&b, &d);
    /// assert_eq!(bca, Some(b)); // B is the best common ancestor
    ///
    /// // Branched history:
    /// //     A
    /// //    / \
    /// //   B   C
    /// let bca = finder.find_best_common_ancestor(&b, &c);
    /// assert_eq!(bca, Some(a)); // A is the best common ancestor
    ///
    /// // Criss-cross merge with multiple BCAs:
    /// //     A
    /// //    / \
    /// //   B   C
    /// //   |\ /|
    /// //   | X |
    /// //   |/ \|
    /// //   D   E
    /// //   |   |
    /// //   F   G
    /// let bca = finder.find_best_common_ancestor(&f, &g);
    /// // Returns either D or E (both are best common ancestors)
    /// ```
    ///
    /// # Performance
    ///
    /// The current implementation may perform redundant work when there are many common ancestors.
    /// Future optimizations could reduce cloning and unnecessary iterations.
    pub fn find_best_common_ancestor(
        &self,
        source_commit_id: &ObjectId,
        target_commit_id: &ObjectId,
    ) -> Option<ObjectId> {
        let target_commit_ids = HashSet::from([target_commit_id]);
        let common_ancestors = self
            .inner
            .find_common_ancestors(source_commit_id, target_commit_ids)
            .into_keys()
            .collect::<HashSet<_>>();

        if common_ancestors.is_empty() {
            return None;
        }

        debug_log!(
            "Found common ancestors: {}",
            common_ancestors
                .iter()
                .map(|oid| oid.as_ref())
                .collect::<Vec<_>>()
                .join(", ")
        );

        // TODO: remove cloning and optimize to avoid unnecessary iterations
        let mut redundant_ancestors = HashSet::<ObjectId>::new();
        for commit in &common_ancestors {
            debug_log!("Evaluating common ancestor {} for redundancy", commit);

            if redundant_ancestors.contains(commit) {
                continue; // Skip already marked redundant ancestors
            }

            let others = common_ancestors
                .iter()
                .filter(|other| *other != commit && !redundant_ancestors.contains(*other))
                .collect::<HashSet<_>>();
            let common_states = self.inner.find_common_ancestors(commit, others.clone());

            if common_states
                .get(commit)
                .unwrap_or(&VisitState::NONE)
                .contains(VisitState::VISITED_FROM_TARGET)
            {
                redundant_ancestors.insert(commit.clone());
            }

            for other in others {
                if common_states
                    .get(other)
                    .unwrap_or(&VisitState::NONE)
                    .contains(VisitState::VISITED_FROM_SOURCE)
                {
                    redundant_ancestors.insert(other.clone());
                }
            }
        }

        debug_log!(
            "Redundant ancestors: {}",
            redundant_ancestors
                .iter()
                .map(|oid| oid.as_ref())
                .collect::<Vec<_>>()
                .join(", ")
        );

        // Filter out redundant ancestors to find the best common ancestor(s)
        let best_common_ancestors = common_ancestors
            .into_iter()
            .filter(|commit| !redundant_ancestors.contains(commit))
            .collect::<Vec<_>>();

        debug_log!(
            "Best common ancestors: {}",
            best_common_ancestors
                .iter()
                .map(|oid| oid.as_ref())
                .collect::<Vec<_>>()
                .join(", ")
        );

        // For simplicity, we return one of the best common ancestors. In a real implementation,
        // we might want to return all of them or apply a recursive approach to find a virtually unique best common ancestor as Git does.
        best_common_ancestors.into_iter().next()
    }
}

#[cfg(test)]
mod tests {
    //! # BCA Finder Test Suite
    //!
    //! This module contains unit tests for the Best Common Ancestor finding algorithm.
    //!
    //! ## Debugging Helpers
    //!
    //! Several debugging utilities are available to help visualize the commit graph
    //! and algorithm execution during test development:
    //!
    //! ### Commit Graph Visualization
    //! ```rust,ignore
    //! let store = InMemoryCommitStore::new();
    //! // ... add commits ...
    //! store.debug_print_graph();  // Print the entire commit graph
    //! store.debug_print_commit(&commit_id);  // Print details of a specific commit
    //! ```
    //!
    //! ### Algorithm State Debugging
    //! ```rust,ignore
    //! // Print common ancestors found
    //! debug_common_ancestors(&source, &target, &ancestors_set);
    //!
    //! // Print the final BCA result
    //! debug_bca_result(&source, &target, result.as_ref());
    //!
    //! // Print internal algorithm states
    //! debug_ancestors_states("After Phase 1", &states_map);
    //! ```
    //!
    //! ### Commit ID Formatting
    //! ```rust,ignore
    //! let readable_name = format_oid(&commit_id);  // Returns "commit_a" instead of hex
    //! ```
    //!
    //! ### Usage in Tests
    //! Simply uncomment the debug lines in any test to see detailed output:
    //! ```bash
    //! cargo test test_name -- --nocapture
    //! ```

    use super::*;
    use chrono::{DateTime, FixedOffset, TimeZone};
    use rstest::*;
    use std::collections::{HashMap, HashSet, VecDeque};

    type CommitData = (ObjectId, Vec<ObjectId>, DateTime<FixedOffset>);
    type CommitGraph = HashMap<ObjectId, CommitData>;

    /// In-memory commit store for testing
    #[derive(Debug, Clone)]
    struct InMemoryCommitStore {
        commits: CommitGraph,
    }

    impl InMemoryCommitStore {
        fn new() -> Self {
            Self {
                commits: HashMap::new(),
            }
        }

        fn add_commit(&mut self, commit_id: ObjectId, parents: Vec<ObjectId>) {
            // Use incrementally increasing timestamps to ensure deterministic ordering
            let timestamp_offset = self.commits.len() as i64 * 3600; // 1 hour apart
            let timestamp = FixedOffset::east_opt(0)
                .unwrap()
                .timestamp_opt(1640995200 + timestamp_offset, 0) // Starting from 2022-01-01
                .unwrap();
            self.commits
                .insert(commit_id.clone(), (commit_id, parents, timestamp));
        }

        fn add_commit_with_timestamp(
            &mut self,
            commit_id: ObjectId,
            parents: Vec<ObjectId>,
            timestamp: DateTime<FixedOffset>,
        ) {
            self.commits
                .insert(commit_id.clone(), (commit_id, parents, timestamp));
        }

        fn get_slim_commit(&'_ self, commit_id: &ObjectId) -> SlimCommit<'_> {
            let (commit_id, parents, timestamp) = self
                .commits
                .get(commit_id)
                .expect("Commit not found in test store");

            SlimCommit {
                oid: commit_id,
                parents: parents.as_slice(),
                timestamp: *timestamp,
            }
        }

        fn get_parents(&self, commit_id: &ObjectId) -> Vec<ObjectId> {
            self.commits
                .get(commit_id)
                .map(|(_, parents, _)| parents.clone())
                .unwrap_or_default()
        }

        /// Print the entire commit graph for debugging
        fn debug_print_graph(&self) {
            let mut all_commits: Vec<_> = self.commits.keys().cloned().collect();
            all_commits.sort_by_key(format_oid);
            print_commit_graph(self, &all_commits);
        }

        /// Print detailed information about a specific commit
        fn debug_print_commit(&self, commit_id: &ObjectId) {
            if let Some((oid, parents, timestamp)) = self.commits.get(commit_id) {
                eprintln!("\n=== Commit Details ===");
                eprintln!("  ID: {}", format_oid(oid));
                eprintln!("  Timestamp: {}", timestamp);
                eprintln!(
                    "  Parents: [{}]",
                    parents
                        .iter()
                        .map(format_oid)
                        .collect::<Vec<_>>()
                        .join(", ")
                );
                eprintln!("======================\n");
            } else {
                eprintln!("\nCommit {} not found in store\n", format_oid(commit_id));
            }
        }
    }

    fn create_oid(id: &str) -> ObjectId {
        // Create a deterministic 40-character hex ObjectId from string for testing
        let mut hex_string = String::new();

        // Use the input string to seed the hex creation
        for byte in id.as_bytes().iter() {
            hex_string.push_str(&format!("{:02x}", byte));
        }

        // Pad or truncate to exactly 40 characters with zeros
        while hex_string.len() < 40 {
            hex_string.push('0');
        }
        hex_string.truncate(40);

        ObjectId::try_parse(hex_string).expect("Invalid test ObjectId")
    }

    /// Format an ObjectId as a short readable string for debugging
    /// Extracts the original commit name from the test OID encoding
    fn format_oid(oid: &ObjectId) -> String {
        let hex = oid.to_string();
        // Try to decode the original name from hex
        let mut name = String::new();
        for i in (0..hex.len()).step_by(2) {
            if i + 1 < hex.len()
                && let Ok(byte) = u8::from_str_radix(&hex[i..i + 2], 16)
            {
                if byte == 0 {
                    break; // Stop at null padding
                }
                if byte.is_ascii_graphic() || byte == b' ' {
                    name.push(byte as char);
                }
            }
        }
        if name.is_empty() {
            format!("{:.7}", hex)
        } else {
            name
        }
    }

    /// Print the commit graph structure for debugging
    fn print_commit_graph(store: &InMemoryCommitStore, commits: &[ObjectId]) {
        eprintln!("\n=== Commit Graph ===");
        for commit_id in commits {
            let parents = store.get_parents(commit_id);
            let parent_names: Vec<String> = parents.iter().map(format_oid).collect();
            if parent_names.is_empty() {
                eprintln!("{} (root)", format_oid(commit_id));
            } else {
                eprintln!("{} <- [{}]", format_oid(commit_id), parent_names.join(", "));
            }
        }
        eprintln!("===================\n");
    }

    /// Print the algorithm's state at each step for debugging
    fn debug_ancestors_states(label: &str, states: &HashMap<ObjectId, VisitState>) {
        eprintln!("\n=== {} ===", label);
        let mut sorted: Vec<_> = states.iter().collect();
        sorted.sort_by_key(|(oid, _)| format_oid(oid));
        for (oid, state) in sorted {
            eprintln!("  {}: {}", format_oid(oid), state);
        }
        eprintln!("===================\n");
    }

    /// Print common ancestors found by the algorithm
    fn debug_common_ancestors(source: &ObjectId, target: &ObjectId, ancestors: &HashSet<ObjectId>) {
        eprintln!(
            "\n=== Common Ancestors: {} <-> {} ===",
            format_oid(source),
            format_oid(target)
        );
        if ancestors.is_empty() {
            eprintln!("  (none found)");
        } else {
            for ancestor in ancestors {
                eprintln!("  {}", format_oid(ancestor));
            }
        }
        eprintln!("===================\n");
    }

    /// Print the best common ancestor result
    fn debug_bca_result(source: &ObjectId, target: &ObjectId, bca: Option<&ObjectId>) {
        eprintln!(
            "\n=== BCA Result: {} <-> {} ===",
            format_oid(source),
            format_oid(target)
        );
        match bca {
            Some(oid) => eprintln!("  Result: {}", format_oid(oid)),
            None => eprintln!("  Result: None"),
        }
        eprintln!("===================\n");
    }

    /// Helper function to check if `ancestor` is an ancestor of `commit` using BFS traversal
    fn is_ancestor_of<'c, F>(commit: &ObjectId, ancestor: &ObjectId, get_parents: &'c F) -> bool
    where
        F: Fn(&ObjectId) -> Option<SlimCommit<'c>>,
    {
        if commit == ancestor {
            return true;
        }

        let mut visited = HashSet::new();
        let mut queue = VecDeque::new();
        queue.push_back(commit.clone());

        while let Some(current) = queue.pop_front() {
            if !visited.insert(current.clone()) {
                continue;
            }

            if &current == ancestor {
                return true;
            }

            if let Some(slim_commit) = get_parents(&current) {
                for parent in slim_commit.parents {
                    queue.push_back(parent.clone());
                }
            }
        }

        false
    }

    /// Helper function to find all common ancestors of two commits
    fn find_all_common_ancestors<'c, F>(
        commit1: &ObjectId,
        commit2: &ObjectId,
        get_parents: &'c F,
    ) -> HashSet<ObjectId>
    where
        F: Fn(&ObjectId) -> Option<SlimCommit<'c>>,
    {
        // Find all ancestors of commit1
        let mut ancestors1 = HashSet::new();
        let mut queue = VecDeque::new();
        queue.push_back(commit1.clone());

        while let Some(current) = queue.pop_front() {
            if !ancestors1.insert(current.clone()) {
                continue;
            }

            if let Some(slim_commit) = get_parents(&current) {
                for parent in slim_commit.parents {
                    queue.push_back(parent.clone());
                }
            }
        }

        // Find all ancestors of commit2
        let mut ancestors2 = HashSet::new();
        queue.push_back(commit2.clone());

        while let Some(current) = queue.pop_front() {
            if !ancestors2.insert(current.clone()) {
                continue;
            }

            if let Some(slim_commit) = get_parents(&current) {
                for parent in slim_commit.parents {
                    queue.push_back(parent.clone());
                }
            }
        }

        // Return intersection
        ancestors1.intersection(&ancestors2).cloned().collect()
    }

    /// Validate that the given ancestor satisfies the best common ancestor invariant:
    /// A best common ancestor of commits X and Y is any common ancestor of X and Y
    /// that is not an ancestor of any other common ancestor.
    fn validate_best_common_ancestor_invariant<'c, F>(
        commit1: &ObjectId,
        commit2: &ObjectId,
        bca: &ObjectId,
        get_parents: &'c F,
    ) -> bool
    where
        F: Fn(&ObjectId) -> Option<SlimCommit<'c>>,
    {
        // BCA must be a common ancestor
        if !is_ancestor_of(commit1, bca, get_parents) || !is_ancestor_of(commit2, bca, get_parents)
        {
            return false;
        }

        // Find all common ancestors
        let common_ancestors = find_all_common_ancestors(commit1, commit2, get_parents);

        // BCA should not be an ancestor of any other common ancestor
        for other_ca in &common_ancestors {
            if other_ca != bca && is_ancestor_of(other_ca, bca, get_parents) {
                return false;
            }
        }

        true
    }

    #[fixture]
    fn linear_history() -> InMemoryCommitStore {
        let mut store = InMemoryCommitStore::new();

        // Linear history: A <- B <- C <- D
        let a = create_oid("commit_a");
        let b = create_oid("commit_b");
        let c = create_oid("commit_c");
        let d = create_oid("commit_d");

        store.add_commit(a.clone(), vec![]); // Initial commit
        store.add_commit(b.clone(), vec![a.clone()]); // B has parent A
        store.add_commit(c.clone(), vec![b.clone()]); // C has parent B
        store.add_commit(d.clone(), vec![c.clone()]); // D has parent C

        store
    }

    #[fixture]
    fn simple_merge() -> InMemoryCommitStore {
        let mut store = InMemoryCommitStore::new();

        //     A
        //    / \
        //   B   C
        //    \ /
        //     D (merge commit)
        let a = create_oid("commit_a");
        let b = create_oid("commit_b");
        let c = create_oid("commit_c");
        let d = create_oid("commit_d");

        store.add_commit(a.clone(), vec![]); // Initial commit
        store.add_commit(b.clone(), vec![a.clone()]); // B has parent A
        store.add_commit(c.clone(), vec![a.clone()]); // C has parent A
        store.add_commit(d.clone(), vec![b.clone(), c.clone()]); // D merges B and C

        store
    }

    #[fixture]
    fn complex_branching() -> InMemoryCommitStore {
        let mut store = InMemoryCommitStore::new();

        //       A
        //      /|\
        //     B C D
        //     | | |\
        //     E F G H
        //      \|/ /
        //       I /
        //        /
        //       J (merge commit)
        let a = create_oid("commit_a");
        let b = create_oid("commit_b");
        let c = create_oid("commit_c");
        let d = create_oid("commit_d");
        let e = create_oid("commit_e");
        let f = create_oid("commit_f");
        let g = create_oid("commit_g");
        let h = create_oid("commit_h");
        let i = create_oid("commit_i");
        let j = create_oid("commit_j");

        store.add_commit(a.clone(), vec![]); // Initial commit
        store.add_commit(b.clone(), vec![a.clone()]); // B has parent A
        store.add_commit(c.clone(), vec![a.clone()]); // C has parent A
        store.add_commit(d.clone(), vec![a.clone()]); // D has parent A
        store.add_commit(e.clone(), vec![b.clone()]); // E has parent B
        store.add_commit(f.clone(), vec![c.clone()]); // F has parent C
        store.add_commit(g.clone(), vec![d.clone()]); // G has parent D
        store.add_commit(h.clone(), vec![d.clone()]); // H has parent D
        store.add_commit(i.clone(), vec![e.clone(), f.clone(), g.clone()]); // I merges E, F, G
        store.add_commit(j.clone(), vec![i.clone(), h.clone()]); // J merges I and H

        store
    }

    #[fixture]
    fn diamond_pattern() -> InMemoryCommitStore {
        let mut store = InMemoryCommitStore::new();

        //     A
        //    /|\
        //   B C D
        //   |X| |
        //   E F G
        //    \|/
        //     H (triple merge)
        let a = create_oid("commit_a");
        let b = create_oid("commit_b");
        let c = create_oid("commit_c");
        let d = create_oid("commit_d");
        let e = create_oid("commit_e");
        let f = create_oid("commit_f");
        let g = create_oid("commit_g");
        let h = create_oid("commit_h");

        store.add_commit(a.clone(), vec![]); // Initial commit
        store.add_commit(b.clone(), vec![a.clone()]); // B has parent A
        store.add_commit(c.clone(), vec![a.clone()]); // C has parent A
        store.add_commit(d.clone(), vec![a.clone()]); // D has parent A
        store.add_commit(e.clone(), vec![b.clone(), c.clone()]); // E merges B and C
        store.add_commit(f.clone(), vec![c.clone(), d.clone()]); // F merges C and D
        store.add_commit(g.clone(), vec![d.clone()]); // G has parent D
        store.add_commit(h.clone(), vec![e.clone(), f.clone(), g.clone()]); // H merges E, F, G

        store
    }

    #[fixture]
    fn criss_cross_merge() -> InMemoryCommitStore {
        let mut store = InMemoryCommitStore::new();

        //     A
        //    / \
        //   B   C
        //   |\  |\
        //   | \ | \
        //   |  \|  \
        //   |   X   |
        //   |  /|   |
        //   | / |   |
        //   D   |   E
        //   |   |   |
        //   F   |   G
        //    \  |  /
        //     \ | /
        //      \|/
        //       H (complex merge)
        let a = create_oid("commit_a");
        let b = create_oid("commit_b");
        let c = create_oid("commit_c");
        let d = create_oid("commit_d");
        let e = create_oid("commit_e");
        let f = create_oid("commit_f");
        let g = create_oid("commit_g");
        let h = create_oid("commit_h");

        store.add_commit(a.clone(), vec![]); // Initial commit
        store.add_commit(b.clone(), vec![a.clone()]); // B has parent A
        store.add_commit(c.clone(), vec![a.clone()]); // C has parent A
        store.add_commit(d.clone(), vec![b.clone(), c.clone()]); // D merges B and C (criss-cross 1)
        store.add_commit(e.clone(), vec![c.clone(), b.clone()]); // E merges C and B (criss-cross 2)
        store.add_commit(f.clone(), vec![d.clone()]); // F has parent D
        store.add_commit(g.clone(), vec![e.clone()]); // G has parent E
        store.add_commit(h.clone(), vec![f.clone(), g.clone()]); // H merges F and G

        store
    }

    #[fixture]
    fn long_parallel_branches() -> InMemoryCommitStore {
        let mut store = InMemoryCommitStore::new();

        //     A
        //    / \
        //   B   C
        //   |   |
        //   D   E
        //   |   |
        //   F   G
        //   |   |
        //   H   I
        //    \ /
        //     J (late merge)
        let a = create_oid("commit_a");
        let b = create_oid("commit_b");
        let c = create_oid("commit_c");
        let d = create_oid("commit_d");
        let e = create_oid("commit_e");
        let f = create_oid("commit_f");
        let g = create_oid("commit_g");
        let h = create_oid("commit_h");
        let i = create_oid("commit_i");
        let j = create_oid("commit_j");

        store.add_commit(a.clone(), vec![]); // Initial commit
        store.add_commit(b.clone(), vec![a.clone()]); // B has parent A
        store.add_commit(c.clone(), vec![a.clone()]); // C has parent A
        store.add_commit(d.clone(), vec![b.clone()]); // D has parent B
        store.add_commit(e.clone(), vec![c.clone()]); // E has parent C
        store.add_commit(f.clone(), vec![d.clone()]); // F has parent D
        store.add_commit(g.clone(), vec![e.clone()]); // G has parent E
        store.add_commit(h.clone(), vec![f.clone()]); // H has parent F
        store.add_commit(i.clone(), vec![g.clone()]); // I has parent G
        store.add_commit(j.clone(), vec![h.clone(), i.clone()]); // J merges H and I

        store
    }

    #[rstest]
    fn test_linear_history_common_ancestor(linear_history: InMemoryCommitStore) {
        let a = create_oid("commit_a");
        let b = create_oid("commit_b");
        let c = create_oid("commit_c");
        let d = create_oid("commit_d");

        let finder = BCAFinder::new(|oid| linear_history.get_slim_commit(oid));

        // Test same commit
        let ancestor = finder.find_best_common_ancestor(&c, &c);
        assert_eq!(ancestor, Some(c));

        // Test linear ancestry
        let ancestor = finder.find_best_common_ancestor(&b, &d);
        assert_eq!(ancestor, Some(b.clone()));

        // Test reverse order
        let ancestor = finder.find_best_common_ancestor(&d, &b);
        assert_eq!(ancestor, Some(b));

        // Test root ancestor
        let ancestor = finder.find_best_common_ancestor(&a, &d);
        assert_eq!(ancestor, Some(a));
    }

    #[rstest]
    fn test_simple_merge_common_ancestor(simple_merge: InMemoryCommitStore) {
        let a = create_oid("commit_a");
        let b = create_oid("commit_b");
        let c = create_oid("commit_c");
        let d = create_oid("commit_d");

        let finder = BCAFinder::new(|oid| simple_merge.get_slim_commit(oid));

        // Test merge commit with its branches
        let ancestor = finder.find_best_common_ancestor(&b, &c);
        assert_eq!(ancestor, Some(a.clone()));

        let ancestor = finder.find_best_common_ancestor(&c, &b);
        assert_eq!(ancestor, Some(a.clone()));

        // Test merge commit with ancestor
        let ancestor = finder.find_best_common_ancestor(&a, &d);
        assert_eq!(ancestor, Some(a));
    }

    #[rstest]
    fn test_complex_branching_common_ancestor(complex_branching: InMemoryCommitStore) {
        let a = create_oid("commit_a");
        let b = create_oid("commit_b");
        let c = create_oid("commit_c");
        let d = create_oid("commit_d");
        let e = create_oid("commit_e");
        let f = create_oid("commit_f");
        let g = create_oid("commit_g");
        let h = create_oid("commit_h");
        let i = create_oid("commit_i");
        let j = create_oid("commit_j");

        let finder = BCAFinder::new(|oid| complex_branching.get_slim_commit(oid));

        eprintln!("\n=== Complex Branching Graph ===");
        complex_branching.debug_print_graph();

        eprintln!("4. Formatted commit IDs:");
        eprintln!("  A: {} -> {}", a, format_oid(&a));
        eprintln!("  B: {} -> {}", b, format_oid(&b));
        eprintln!("  C: {} -> {}", c, format_oid(&c));
        eprintln!("  D: {} -> {}", d, format_oid(&d));
        eprintln!("  E: {} -> {}", e, format_oid(&e));
        eprintln!("  F: {} -> {}", f, format_oid(&f));
        eprintln!("  G: {} -> {}", g, format_oid(&g));
        eprintln!("  H: {} -> {}", h, format_oid(&h));
        eprintln!("  I: {} -> {}", i, format_oid(&i));
        eprintln!("  J: {} -> {}", j, format_oid(&j));
        eprintln!("==============================\n");

        // Test different branch tips
        let ancestor = finder.find_best_common_ancestor(&e, &f);
        assert_eq!(
            ancestor,
            Some(a.clone()),
            "E and F should have A as best common ancestor"
        );

        // Test complex merge scenarios
        let ancestor = finder.find_best_common_ancestor(&i, &h);
        assert_eq!(
            ancestor,
            Some(d.clone()),
            "I and H should have D as best common ancestor"
        );

        // Test final merge with branch - J has I and H as parents, I merges E, F, G
        // The BCA algorithm should find the best common ancestor
        let ancestor_jg = finder.find_best_common_ancestor(&j, &g);
        assert!(
            ancestor_jg.is_some(),
            "Should find a best common ancestor for J and G"
        );

        // Test distant commits
        let ancestor = finder.find_best_common_ancestor(&e, &h);
        assert_eq!(
            ancestor,
            Some(a),
            "E and H should have A as best common ancestor"
        );
    }

    #[rstest]
    fn test_diamond_pattern_common_ancestor(diamond_pattern: InMemoryCommitStore) {
        let a = create_oid("commit_a");
        let c = create_oid("commit_c");
        let e = create_oid("commit_e");
        let f = create_oid("commit_f");
        let g = create_oid("commit_g");
        let h = create_oid("commit_h");

        let finder = BCAFinder::new(|oid| diamond_pattern.get_slim_commit(oid));

        // Test cross-merges
        let ancestor = finder.find_best_common_ancestor(&e, &f);
        assert_eq!(ancestor, Some(c)); // E and F share C as best common ancestor

        // Test with isolated branch
        let ancestor = finder.find_best_common_ancestor(&e, &g);
        assert_eq!(ancestor, Some(a));

        // Test triple merge scenarios
        let ancestor = finder.find_best_common_ancestor(&h, &e);
        assert_eq!(ancestor, Some(e));

        let ancestor = finder.find_best_common_ancestor(&h, &f);
        assert_eq!(ancestor, Some(f));

        let ancestor = finder.find_best_common_ancestor(&h, &g);
        assert_eq!(ancestor, Some(g));
    }

    #[rstest]
    fn test_criss_cross_merge_common_ancestor(criss_cross_merge: InMemoryCommitStore) {
        let a = create_oid("commit_a");
        let b = create_oid("commit_b");
        let c = create_oid("commit_c");
        let d = create_oid("commit_d");
        let e = create_oid("commit_e");

        // Debug the commit graph structure for this complex scenario
        eprintln!("\n=== Criss-Cross Merge Object IDs ===");
        eprintln!("A: {}", a);
        eprintln!("B: {}", b);
        eprintln!("C: {}", c);
        eprintln!("D: {}", d);
        eprintln!("E: {}", e);
        eprintln!("===============================\n");

        let finder = BCAFinder::new(|oid| criss_cross_merge.get_slim_commit(oid));

        criss_cross_merge.debug_print_graph();

        // Test criss-cross merge points - D and E are merge commits from B and C
        // Both D and E are best common ancestors (neither is ancestor of the other)
        let ancestor = finder.find_best_common_ancestor(&d, &e);
        let ancestor_value = ancestor.unwrap();
        assert!(
            ancestor_value == b || ancestor_value == c,
            "Expected B or C as best common ancestor for D and E, got {:?}",
            ancestor_value
        );

        // Test final merge with branches - F comes from D, G comes from E
        // Since D = merge(B, C) and E = merge(C, B), the common ancestors of F and G are B, C, and A
        // The best common ancestors are B and C (not ancestors of each other)
        let f = create_oid("commit_f");
        let g = create_oid("commit_g");
        let ancestor = finder.find_best_common_ancestor(&f, &g);
        let ancestor_value = ancestor.unwrap();
        assert!(
            ancestor_value == b || ancestor_value == c,
            "Expected B or C as best common ancestor, got {:?}",
            ancestor_value
        );

        // Test complex merge point - H has F as parent, which has D as parent
        let h = create_oid("commit_h");
        let ancestor = finder.find_best_common_ancestor(&h, &d);
        assert_eq!(ancestor, Some(d));
    }

    #[rstest]
    fn test_long_parallel_branches_common_ancestor(long_parallel_branches: InMemoryCommitStore) {
        let a = create_oid("commit_a");
        let h = create_oid("commit_h");
        let i = create_oid("commit_i");
        let j = create_oid("commit_j");

        let finder = BCAFinder::new(|oid| long_parallel_branches.get_slim_commit(oid));

        // Test long parallel branches
        let ancestor = finder.find_best_common_ancestor(&h, &i);
        assert_eq!(ancestor, Some(a.clone()));

        // Test merge point with branches
        let ancestor = finder.find_best_common_ancestor(&j, &h);
        assert_eq!(ancestor, Some(h.clone()));

        let ancestor = finder.find_best_common_ancestor(&j, &i);
        assert_eq!(ancestor, Some(i));

        // Test root relationship
        let ancestor = finder.find_best_common_ancestor(&h, &a);
        assert_eq!(ancestor, Some(a));
    }

    #[rstest]
    fn test_no_common_ancestor_different_roots() {
        let mut store = InMemoryCommitStore::new();

        // Two separate trees with no common history
        let a = create_oid("commit_a");
        let b = create_oid("commit_b");
        let x = create_oid("commit_x");
        let y = create_oid("commit_y");

        store.add_commit(a.clone(), vec![]); // Root 1
        store.add_commit(b.clone(), vec![a.clone()]); // Child of root 1
        store.add_commit(x.clone(), vec![]); // Root 2 (separate)
        store.add_commit(y.clone(), vec![x.clone()]); // Child of root 2

        // Debug helpers (uncomment to see detailed output during test runs):
        // store.debug_print_graph();
        // store.debug_print_commit(&b);
        // store.debug_print_commit(&y);

        let finder = BCAFinder::new(|oid| store.get_slim_commit(oid));

        // For commits with no common history, the algorithm should return None
        let result = finder.find_best_common_ancestor(&b, &y);

        // Debug the result (uncomment to see):
        // debug_bca_result(&b, &y, result.as_ref());

        assert_eq!(
            result, None,
            "Expected None for commits with no common ancestor"
        );
    }

    #[rstest]
    fn test_single_commit_repository() {
        let mut store = InMemoryCommitStore::new();
        let a = create_oid("commit_a");
        store.add_commit(a.clone(), vec![]);

        let finder = BCAFinder::new(|oid| store.get_slim_commit(oid));

        // Single commit should be its own best common ancestor
        let ancestor = finder.find_best_common_ancestor(&a, &a);
        assert_eq!(ancestor, Some(a));
    }

    #[rstest]
    fn test_parent_child_relationship() {
        let mut store = InMemoryCommitStore::new();
        let parent = create_oid("parent");
        let child = create_oid("child");

        store.add_commit(parent.clone(), vec![]);
        store.add_commit(child.clone(), vec![parent.clone()]);

        let finder = BCAFinder::new(|oid| store.get_slim_commit(oid));

        // Parent should be the best common ancestor of parent and child
        let ancestor = finder.find_best_common_ancestor(&parent, &child);
        assert_eq!(ancestor, Some(parent.clone()));

        // Order shouldn't matter
        let ancestor = finder.find_best_common_ancestor(&child, &parent);
        assert_eq!(ancestor, Some(parent));
    }

    #[rstest]
    fn test_octopus_merge_scenario() {
        let mut store = InMemoryCommitStore::new();

        // Octopus merge: one commit merging multiple branches
        //     A
        //   / | \
        //  B  C  D
        //   \|/|/
        //    E (octopus merge)
        let a = create_oid("commit_a");
        let b = create_oid("commit_b");
        let c = create_oid("commit_c");
        let d = create_oid("commit_d");
        let e = create_oid("commit_e");

        store.add_commit(a.clone(), vec![]); // Initial commit
        store.add_commit(b.clone(), vec![a.clone()]); // B has parent A
        store.add_commit(c.clone(), vec![a.clone()]); // C has parent A
        store.add_commit(d.clone(), vec![a.clone()]); // D has parent A
        store.add_commit(e.clone(), vec![b.clone(), c.clone(), d.clone()]); // E merges B, C, and D (octopus)

        let finder = BCAFinder::new(|oid| store.get_slim_commit(oid));

        // Test octopus merge with its parents
        let ancestor = finder.find_best_common_ancestor(&e, &b);
        assert_eq!(ancestor, Some(b.clone()));

        let ancestor = finder.find_best_common_ancestor(&e, &c);
        assert_eq!(ancestor, Some(c.clone()));

        let ancestor = finder.find_best_common_ancestor(&e, &d);
        assert_eq!(ancestor, Some(d));

        // Test branches against each other
        let ancestor = finder.find_best_common_ancestor(&b, &c);
        assert_eq!(ancestor, Some(a));
    }

    #[rstest]
    fn test_multiple_common_ancestors_looping_history() {
        let mut store = InMemoryCommitStore::new();

        // Classic looping history with multiple common ancestors where none is ancestor of the other
        // This scenario creates multiple best common ancestors
        //
        //     A (root)
        //    / \
        //   B   C
        //   |\ /|
        //   | X |   (B and C are merged bidirectionally)
        //   |/ \|
        //   D   E
        //   |   |
        //   F   G
        //
        // When comparing F and G, both D and E are common ancestors,
        // but neither D nor E is an ancestor of the other, making both "best" common ancestors
        let a = create_oid("commit_a");
        let b = create_oid("commit_b");
        let c = create_oid("commit_c");
        let d = create_oid("commit_d");
        let e = create_oid("commit_e");
        let f = create_oid("commit_f");
        let g = create_oid("commit_g");

        store.add_commit(a.clone(), vec![]); // Initial commit
        store.add_commit(b.clone(), vec![a.clone()]); // B has parent A
        store.add_commit(c.clone(), vec![a.clone()]); // C has parent A
        store.add_commit(d.clone(), vec![b.clone(), c.clone()]); // D merges B and C
        store.add_commit(e.clone(), vec![c.clone(), b.clone()]); // E merges C and B (reverse order)
        store.add_commit(f.clone(), vec![d.clone()]); // F has parent D
        store.add_commit(g.clone(), vec![e.clone()]); // G has parent E

        let finder = BCAFinder::new(|oid| store.get_slim_commit(oid));

        // When finding best common ancestor of F and G, the algorithm finds B and C
        // as both are best common ancestors (neither is ancestor of the other, both are parents of D and E)
        let ancestor = finder.find_best_common_ancestor(&f, &g);
        let ancestor_value = ancestor.unwrap();

        // The result should be one of the best common ancestors (B or C)
        assert!(
            ancestor_value == b || ancestor_value == c,
            "Expected B or C as best common ancestor, got {:?}",
            ancestor_value
        );
    }

    #[rstest]
    fn test_best_common_ancestor_invariant() {
        let mut store = InMemoryCommitStore::new();

        // Test the invariant: A best common ancestor of commits X and Y is any common
        // ancestor of X and Y that is not an ancestor of any other common ancestor.
        //
        //     A (root)
        //    / \
        //   B   C
        //   |   |
        //   D   E
        //   |\ /|
        //   | X |   (criss-cross merge)
        //   |/ \|
        //   F   G
        //   |   |
        //   H   I
        //
        // When comparing H and I:
        // - Common ancestors include: A, B, C, D, E, F, G
        // - A is ancestor of both B and C, so A is NOT a best common ancestor
        // - B is ancestor of D, so B is NOT a best common ancestor
        // - C is ancestor of E, so C is NOT a best common ancestor
        // - D is ancestor of F, so D is NOT a best common ancestor
        // - E is ancestor of G, so E is NOT a best common ancestor
        // - F and G are NOT ancestors of each other, so BOTH are best common ancestors
        let a = create_oid("commit_a");
        let b = create_oid("commit_b");
        let c = create_oid("commit_c");
        let d = create_oid("commit_d");
        let e = create_oid("commit_e");
        let f = create_oid("commit_f");
        let g = create_oid("commit_g");
        let h = create_oid("commit_h");
        let i = create_oid("commit_i");

        store.add_commit(a.clone(), vec![]); // Initial commit
        store.add_commit(b.clone(), vec![a.clone()]); // B has parent A
        store.add_commit(c.clone(), vec![a.clone()]); // C has parent A
        store.add_commit(d.clone(), vec![b.clone()]); // D has parent B
        store.add_commit(e.clone(), vec![c.clone()]); // E has parent C
        store.add_commit(f.clone(), vec![d.clone(), e.clone()]); // F merges D and E
        store.add_commit(g.clone(), vec![e.clone(), d.clone()]); // G merges E and D (reverse)
        store.add_commit(h.clone(), vec![f.clone()]); // H has parent F
        store.add_commit(i.clone(), vec![g.clone()]); // I has parent G

        let finder = BCAFinder::new(|oid| store.get_slim_commit(oid));

        // When finding best common ancestor of H and I, we should get D or E (best common ancestors)
        // F and G are NOT common ancestors (they're on separate branches)
        // Among A, B, C, D, E: D and E are best (neither is ancestor of the other)
        let ancestor = finder.find_best_common_ancestor(&h, &i);
        let ancestor_value = ancestor.unwrap();

        // The result should be one of the best common ancestors (D or E)
        assert!(
            ancestor_value == d || ancestor_value == e,
            "Expected D or E as best common ancestor, got {:?}",
            ancestor_value
        );

        // Verify it's NOT one of the non-best common ancestors
        assert!(
            ancestor_value != a && ancestor_value != b && ancestor_value != c,
            "Got non-best common ancestor: {:?}",
            ancestor_value
        );
    }

    #[rstest]
    fn test_merge_commit_as_common_ancestor() {
        let mut store = InMemoryCommitStore::new();

        // Test scenario where a merge commit itself is the best common ancestor
        //
        //     A
        //    / \
        //   B   C
        //    \ /
        //     D (merge commit)
        //    / \
        //   E   F
        //   |   |
        //   G   H
        //
        // When comparing G and H, D is the best common ancestor (it's a merge commit)
        let a = create_oid("commit_a");
        let b = create_oid("commit_b");
        let c = create_oid("commit_c");
        let d = create_oid("commit_d");
        let e = create_oid("commit_e");
        let f = create_oid("commit_f");
        let g = create_oid("commit_g");
        let h = create_oid("commit_h");

        store.add_commit(a.clone(), vec![]); // Initial commit
        store.add_commit(b.clone(), vec![a.clone()]); // B has parent A
        store.add_commit(c.clone(), vec![a.clone()]); // C has parent A
        store.add_commit(d.clone(), vec![b.clone(), c.clone()]); // D merges B and C
        store.add_commit(e.clone(), vec![d.clone()]); // E has parent D
        store.add_commit(f.clone(), vec![d.clone()]); // F has parent D
        store.add_commit(g.clone(), vec![e.clone()]); // G has parent E
        store.add_commit(h.clone(), vec![f.clone()]); // H has parent F

        let finder = BCAFinder::new(|oid| store.get_slim_commit(oid));

        // D (a merge commit) should be the best common ancestor of G and H
        let ancestor = finder.find_best_common_ancestor(&g, &h);
        assert_eq!(
            ancestor,
            Some(d),
            "Merge commit D should be the best common ancestor"
        );
    }

    #[rstest]
    fn test_complex_looping_history_with_multiple_merges() {
        let mut store = InMemoryCommitStore::new();

        // Complex looping history with multiple interconnected merges
        //
        //       A (root)
        //      / \
        //     B   C
        //     |\ /|
        //     | X |  (first criss-cross)
        //     |/ \|
        //     D   E
        //     |\ /|
        //     | X |  (second criss-cross)
        //     |/ \|
        //     F   G
        //     |   |
        //     H   I
        //
        // When comparing H and I, F and G are the best common ancestors
        let a = create_oid("commit_a");
        let b = create_oid("commit_b");
        let c = create_oid("commit_c");
        let d = create_oid("commit_d");
        let e = create_oid("commit_e");
        let f = create_oid("commit_f");
        let g = create_oid("commit_g");
        let h = create_oid("commit_h");
        let i = create_oid("commit_i");

        store.add_commit(a.clone(), vec![]); // Initial commit
        store.add_commit(b.clone(), vec![a.clone()]); // B has parent A
        store.add_commit(c.clone(), vec![a.clone()]); // C has parent A
        store.add_commit(d.clone(), vec![b.clone(), c.clone()]); // D merges B and C
        store.add_commit(e.clone(), vec![c.clone(), b.clone()]); // E merges C and B
        store.add_commit(f.clone(), vec![d.clone(), e.clone()]); // F merges D and E
        store.add_commit(g.clone(), vec![e.clone(), d.clone()]); // G merges E and D
        store.add_commit(h.clone(), vec![f.clone()]); // H has parent F
        store.add_commit(i.clone(), vec![g.clone()]); // I has parent G

        let finder = BCAFinder::new(|oid| store.get_slim_commit(oid));

        // F and G are both best common ancestors (neither is ancestor of the other)
        let ancestor = finder.find_best_common_ancestor(&h, &i);
        assert!(ancestor.is_some(), "Should find a best common ancestor");

        let ancestor_value = ancestor.unwrap();

        // Validate that the returned ancestor satisfies the BCA invariant
        let is_valid = validate_best_common_ancestor_invariant(&h, &i, &ancestor_value, &|oid| {
            Some(store.get_slim_commit(oid))
        });

        assert!(
            is_valid,
            "Returned ancestor {:?} does not satisfy the best common ancestor invariant",
            ancestor_value
        );
    }

    #[rstest]
    fn test_asymmetric_merge_history() {
        let mut store = InMemoryCommitStore::new();

        // Asymmetric merge history where one branch has more activity
        //
        //     A
        //    / \
        //   B   C
        //   |   |\
        //   |   | D
        //   |   |/
        //   |   E (merge)
        //   |   |\
        //   |   | F
        //   |   |/
        //   |   G (merge)
        //   |   |
        //   H   I
        //    \ /
        //     J (final merge)
        //
        // When comparing branches before final merge, we should get correct best common ancestor
        let a = create_oid("commit_a");
        let b = create_oid("commit_b");
        let c = create_oid("commit_c");
        let d = create_oid("commit_d");
        let e = create_oid("commit_e");
        let f = create_oid("commit_f");
        let g = create_oid("commit_g");
        let h = create_oid("commit_h");
        let i = create_oid("commit_i");
        let j = create_oid("commit_j");

        store.add_commit(a.clone(), vec![]); // Initial commit
        store.add_commit(b.clone(), vec![a.clone()]); // B has parent A
        store.add_commit(c.clone(), vec![a.clone()]); // C has parent A
        store.add_commit(d.clone(), vec![c.clone()]); // D has parent C
        store.add_commit(e.clone(), vec![c.clone(), d.clone()]); // E merges C and D
        store.add_commit(f.clone(), vec![e.clone()]); // F has parent E
        store.add_commit(g.clone(), vec![e.clone(), f.clone()]); // G merges E and F
        store.add_commit(h.clone(), vec![b.clone()]); // H has parent B
        store.add_commit(i.clone(), vec![g.clone()]); // I has parent G
        store.add_commit(j.clone(), vec![h.clone(), i.clone()]); // J merges H and I

        let finder = BCAFinder::new(|oid| store.get_slim_commit(oid));

        // A should be the best common ancestor of H and I (simple case)
        let ancestor = finder.find_best_common_ancestor(&h, &i);
        assert_eq!(
            ancestor,
            Some(a),
            "A should be the best common ancestor of H and I"
        );
    }

    #[rstest]
    fn test_triple_criss_cross_merge() {
        let mut store = InMemoryCommitStore::new();

        // Triple criss-cross merge pattern
        //
        //       A
        //     / | \
        //    B  C  D
        //    |\/|\/|
        //    |/\|/\|
        //    E  F  G  (all three interconnected via merges)
        //    |  |  |
        //    H  I  J
        //
        // E merges B and C, F merges C and D, G merges D and B
        let a = create_oid("commit_a");
        let b = create_oid("commit_b");
        let c = create_oid("commit_c");
        let d = create_oid("commit_d");
        let e = create_oid("commit_e");
        let f = create_oid("commit_f");
        let g = create_oid("commit_g");
        let h = create_oid("commit_h");
        let i = create_oid("commit_i");
        let j = create_oid("commit_j");

        store.add_commit(a.clone(), vec![]); // Initial commit
        store.add_commit(b.clone(), vec![a.clone()]); // B has parent A
        store.add_commit(c.clone(), vec![a.clone()]); // C has parent A
        store.add_commit(d.clone(), vec![a.clone()]); // D has parent A
        store.add_commit(e.clone(), vec![b.clone(), c.clone()]); // E merges B and C
        store.add_commit(f.clone(), vec![c.clone(), d.clone()]); // F merges C and D
        store.add_commit(g.clone(), vec![d.clone(), b.clone()]); // G merges D and B
        store.add_commit(h.clone(), vec![e.clone()]); // H has parent E
        store.add_commit(i.clone(), vec![f.clone()]); // I has parent F
        store.add_commit(j.clone(), vec![g.clone()]); // J has parent G

        let store_for_validation = store.clone();
        let finder = BCAFinder::new(|oid| store.get_slim_commit(oid));

        // Test H and I: validate BCA invariant
        let ancestor_hi = finder.find_best_common_ancestor(&h, &i);
        assert!(
            ancestor_hi.is_some(),
            "Should find a best common ancestor for H and I"
        );

        let ancestor_hi_value = ancestor_hi.unwrap();
        let is_valid =
            validate_best_common_ancestor_invariant(&h, &i, &ancestor_hi_value, &|oid| {
                Some(store_for_validation.get_slim_commit(oid))
            });
        assert!(
            is_valid,
            "Returned ancestor {:?} for H and I does not satisfy the best common ancestor invariant",
            ancestor_hi_value
        );

        // Test I and J: validate BCA invariant
        let ancestor_ij = finder.find_best_common_ancestor(&i, &j);
        assert!(
            ancestor_ij.is_some(),
            "Should find a best common ancestor for I and J"
        );

        let ancestor_ij_value = ancestor_ij.unwrap();
        let is_valid =
            validate_best_common_ancestor_invariant(&i, &j, &ancestor_ij_value, &|oid| {
                Some(store_for_validation.get_slim_commit(oid))
            });
        assert!(
            is_valid,
            "Returned ancestor {:?} for I and J does not satisfy the best common ancestor invariant",
            ancestor_ij_value
        );

        // Test H and J: validate BCA invariant
        let ancestor_hj = finder.find_best_common_ancestor(&h, &j);
        assert!(
            ancestor_hj.is_some(),
            "Should find a best common ancestor for H and J"
        );

        let ancestor_hj_value = ancestor_hj.unwrap();
        let is_valid =
            validate_best_common_ancestor_invariant(&h, &j, &ancestor_hj_value, &|oid| {
                Some(store_for_validation.get_slim_commit(oid))
            });
        assert!(
            is_valid,
            "Returned ancestor {:?} for H and J does not satisfy the best common ancestor invariant",
            ancestor_hj_value
        );
    }

    // ============================================================================
    // Tests below validate edge cases and specific scenarios
    // ============================================================================

    #[rstest]
    fn test_bca_invariant_commit_and_its_ancestor() {
        let mut store = InMemoryCommitStore::new();

        //       A
        //      /|\
        //     B C D
        //     | | |\
        //     E F G H
        //      \|/ /
        //       I /
        //        /
        //       J (merge commit)
        let a = create_oid("commit_a");
        let d = create_oid("commit_d");
        let g = create_oid("commit_g");
        let i = create_oid("commit_i");
        let j = create_oid("commit_j");

        store.add_commit(a.clone(), vec![]); // Initial commit
        store.add_commit(d.clone(), vec![a.clone()]); // D has parent A
        store.add_commit(g.clone(), vec![d.clone()]); // G has parent D
        store.add_commit(i.clone(), vec![g.clone()]); // I has G as one parent (simplified)
        store.add_commit(j.clone(), vec![i.clone(), d.clone()]); // J merges I and D's other child

        let store_for_validation = store.clone();
        let finder = BCAFinder::new(|oid| store.get_slim_commit(oid));

        // When one commit is an ancestor of the other, BCA should be the ancestor
        let ancestor = finder.find_best_common_ancestor(&j, &g);
        assert!(ancestor.is_some());

        let ancestor_value = ancestor.unwrap();
        let is_valid = validate_best_common_ancestor_invariant(&j, &g, &ancestor_value, &|oid| {
            Some(store_for_validation.get_slim_commit(oid))
        });

        assert!(is_valid, "Should satisfy BCA invariant");
        assert_eq!(ancestor_value, g, "BCA should be G (the ancestor) not D");
    }

    /// Example test demonstrating all debug helpers
    /// Run with: cargo test test_debug_helpers_example -- --nocapture
    #[test]
    #[ignore] // Ignored by default since it's just for demonstration
    fn test_debug_helpers_example() {
        let mut store = InMemoryCommitStore::new();

        // Create a simple merge scenario:
        //     A
        //    / \
        //   B   C
        //    \ /
        //     D
        let a = create_oid("commit_a");
        let b = create_oid("commit_b");
        let c = create_oid("commit_c");
        let d = create_oid("commit_d");

        store.add_commit(a.clone(), vec![]);
        store.add_commit(b.clone(), vec![a.clone()]);
        store.add_commit(c.clone(), vec![a.clone()]);
        store.add_commit(d.clone(), vec![b.clone(), c.clone()]);

        eprintln!("\n{}", "=".repeat(60));
        eprintln!("DEBUG HELPERS DEMONSTRATION");
        eprintln!("{}\n", "=".repeat(60));

        // 1. Print the entire graph
        eprintln!("1. Visualizing the commit graph:");
        store.debug_print_graph();

        // 2. Print individual commit details
        eprintln!("2. Detailed information about commit D:");
        store.debug_print_commit(&d);

        // 3. Test the BCA finder
        let finder = BCAFinder::new(|oid| store.get_slim_commit(oid));

        eprintln!("3. Finding BCA between B and C:");
        let result = finder.find_best_common_ancestor(&b, &c);

        // 4. Print the result
        debug_bca_result(&b, &c, result.as_ref());

        // 5. Show formatted OIDs
        eprintln!("4. Formatted commit IDs:");
        eprintln!("  A: {} -> {}", a, format_oid(&a));
        eprintln!("  B: {} -> {}", b, format_oid(&b));
        eprintln!("  C: {} -> {}", c, format_oid(&c));
        eprintln!("  D: {} -> {}", d, format_oid(&d));

        eprintln!("\n{}\n", "=".repeat(60));

        // Assertion to make the test pass
        assert_eq!(result, Some(a), "A should be the BCA of B and C");
    }
}
