# Log Merge Commit Traversal Design

## Mission

Design how `bit log` must traverse merge commits to match Git's behavior while maintaining correctness guarantees.

## Git Semantics: How Real Git Handles Merge Commits

### Core Behavior

1. **Default traversal**: Git log traverses ALL parents of merge commits
   - Priority queue ordered by timestamp
   - Each parent is enqueued and visited
   - This is NOT first-parent traversal by default

2. **First-parent mode**: `git log --first-parent`
   - Follow ONLY the first parent when encountering merge commits
   - Skips the entire history of merged branches
   - Used to view the "main line" of development

3. **Merge commit structure**:
   ```
   tree <sha>
   parent <first-parent-sha>    # The branch you were on when merging
   parent <second-parent-sha>   # The branch you merged in
   parent <third-parent-sha>    # (for octopus merges)
   author ...
   committer ...
   ```

### Parent Ordering Semantics

The order of parents in a merge commit is semantically meaningful:

- **First parent** = the current branch (where you ran `git merge`)
- **Second parent** = the branch being merged in
- **Additional parents** = octopus merge participants

**Example**:
```
# On branch main
git merge feature

# Creates merge commit with:
# parent <main-tip>     # first parent
# parent <feature-tip>  # second parent
```

### Traversal Algorithm

Git's default log uses a **priority queue with ALL parents enqueued**:

1. Start with HEAD commit in priority queue (ordered by timestamp, newest first)
2. Pop commit from queue
3. Display the commit (if interesting)
4. **Enqueue ALL parents** to the priority queue
5. Mark commit as visited (to avoid duplicates)
6. Repeat until queue empty

The result is **timestamp-ordered traversal of the entire reachable DAG**.

### Graph vs Non-Graph Mode

- **Without `--graph`**: timestamp ordering, no special merge handling
- **With `--graph`**: topological ordering required for proper display
  - Must maintain parent relationships
  - Must draw graph lines correctly

## Current Bit Implementation Issues

Looking at `/Users/sami/Desktop/Development/bit/src/artifacts/log/rev_list.rs`:

### Current Behavior
```rust
fn add_parent(&mut self, oid: &ObjectId) -> anyhow::Result<()> {
    // ...
    
    // Load its parent into the cache if not already present
    if let Some(parent_oid) = commit.parent() {  // ⚠️ ONLY FIRST PARENT
        self.load_commit(parent_oid)?;
        // ...
        self.enqueue_commit(parent_oid)?;
    }
    
    // ...
}
```

**Problem**: `commit.parent()` returns only the **first parent**, implementing first-parent traversal by default, which is WRONG.

### What Needs To Change

The `Commit` struct has:
```rust
pub struct Commit {
    parents: Vec<ObjectId>,  // ✓ Supports multiple parents
    // ...
}

impl Commit {
    pub fn parent(&self) -> Option<&ObjectId> {
        self.parents.first()  // ⚠️ Only returns first
    }
}
```

We need:
```rust
impl Commit {
    pub fn parents(&self) -> &[ObjectId] {
        &self.parents
    }
}
```

## Design: Correct Merge Commit Traversal

### Data Structures

**No changes needed** to core structures. The priority queue already exists:

```rust
pub struct RevList<'r> {
    commits_pqueue: BinaryHeap<CommitQueueEntry>,  // ✓ Priority queue
    commits_cache: HashMap<ObjectId, Commit>,       // ✓ Commit cache
    commits_flags: HashMap<ObjectId, HashSet<LogTraversalCommitFlag>>,  // ✓ Visited tracking
    // ...
}
```

### Algorithm: Default Traversal (All Parents)

```
FUNCTION traverse_all_parents(starting_revisions):
    1. Initialize priority queue with starting revisions
    2. Initialize visited set (using commits_flags with Seen/Added)
    
    3. WHILE priority queue not empty:
        a. Pop commit C with newest timestamp
        b. IF C already has Added flag: SKIP (already processed)
        c. Mark C with Added flag
        d. Output C (or add to output list if limited mode)
        
        e. FOR EACH parent P in C.parents():  ← KEY: ALL parents
            i.   Load P from database if needed
            ii.  IF P not marked Seen:
                    - Mark P as Seen
                    - Enqueue P to priority queue
            iii. Handle uninteresting propagation if limited mode
            
    4. Return commits in timestamp order
```

### Algorithm: First-Parent Traversal

For future `--first-parent` support:

```
FUNCTION traverse_first_parent(starting_revision):
    1. current = starting_revision
    2. WHILE current is not None:
        a. Output current
        b. current = current.parent() (first parent only)
```

This is a linear walk, not a priority queue traversal.

### Invariants To Maintain

1. **DAG property**: History is a directed acyclic graph
   - Commits reference parents, never children
   - Object IDs are content-addressed

2. **Visited tracking**: Each commit processed exactly once
   - Use `LogTraversalCommitFlag::Added` to prevent re-processing
   - Prevents infinite loops in broken repos (should never happen with SHA-1)

3. **Timestamp ordering**: Priority queue maintains chronological order
   - Uses `BinaryHeap<CommitQueueEntry>` ordered by timestamp
   - Tiebreaker on ObjectId for determinism

4. **No duplication**: Each commit appears once in output
   - Diamond merges: commit reachable via multiple paths
   - Visit tracking ensures single appearance

5. **Parent order preservation**: 
   - Must iterate `commit.parents()` in order
   - First parent = current branch lineage
   - Critical for future first-parent mode

### Edge Cases

1. **Octopus merge**: Commit with 3+ parents
   - Enqueue ALL parents
   - Each gets traversed independently

2. **Diamond merge**:
   ```
       A
      / \
     B   C
      \ /
       D
   ```
   - A reachable from D via both B and C
   - Must appear only once in output
   - Priority queue + visited flags handle this

3. **Criss-cross merge**: Multiple merge bases
   - Traversal unaffected
   - BCA finder handles this separately

4. **Identical timestamps**:
   - Tiebreaker on ObjectId maintains stable order
   - Already implemented in `CommitQueueEntry::Ord`

5. **Interleaved histories**:
   ```
   Time:  T5   T4   T3   T2   T1
   Main:  M2 ← M1 ← 
   Feat:            ← F2 ← F1 ←
   Base:                       B
   ```
   Output order: M2, F2, M1, F1, B (by timestamp)

### Limited Mode (Range Expressions / Path Filtering)

**Current behavior**: Already correct for range expressions.

The `mark_parents_uninteresting` function walks ONLY the first parent:
```rust
fn mark_parents_uninteresting(&mut self, oid: &ObjectId) -> anyhow::Result<()> {
    let mut commit = ...;
    
    while let Some(parent_oid) = commit.parent()  // ⚠️ First parent only
        && let Some(parent_commit) = self.commits_cache.get(parent_oid)
    {
        // Mark uninteresting and continue
        commit = parent_commit;
    }
}
```

**Problem**: This assumes linear history for exclusions.

**Fix needed**: For correct range exclusion:
```rust
fn mark_parents_uninteresting(&mut self, oid: &ObjectId) -> anyhow::Result<()> {
    let mut queue = VecDeque::new();
    queue.push_back(oid.clone());
    
    while let Some(commit_oid) = queue.pop_front() {
        if !self.commits_flags
            .entry(commit_oid.clone())
            .or_default()
            .insert(LogTraversalCommitFlag::Uninteresting)
        {
            continue; // Already marked
        }
        
        if let Some(commit) = self.commits_cache.get(&commit_oid) {
            for parent in commit.parents() {  // ALL parents
                self.load_commit(parent)?;
                queue.push_back(parent.clone());
            }
        }
    }
    
    Ok(())
}
```

This ensures that `git log A..B` excludes ALL commits reachable from A, not just first-parent lineage.

## Implementation Changes Required

### 1. Add `parents()` accessor to `Commit`

**File**: `src/artifacts/objects/commit.rs`

```rust
impl Commit {
    /// Get all parent commit IDs
    ///
    /// Returns empty slice for root commits, one parent for normal commits,
    /// multiple parents for merge commits.
    pub fn parents(&self) -> &[ObjectId] {
        &self.parents
    }
    
    /// Get the first parent (for convenience)
    ///
    /// Returns None for root commits. For merge commits, this is the
    /// "current branch" parent (the branch you were on when merging).
    pub fn parent(&self) -> Option<&ObjectId> {
        self.parents.first()
    }
}
```

### 1b. **CRITICAL**: Fix `CommitCache::load_commit` to store all parents

**File**: `src/areas/database.rs` (around line 462)

**Current (BUGGY)**:
```rust
let cached = CachedCommit {
    oid: commit.object_id()?,
    parents: commit.parent().cloned().into_iter().collect(),  // ⚠️ Only first parent!
    timestamp: commit.timestamp(),
};
```

**Fixed**:
```rust
let cached = CachedCommit {
    oid: commit.object_id()?,
    parents: commit.parents().to_vec(),  // ✓ All parents
    timestamp: commit.timestamp(),
};
```

**Impact**: This bug would break BCA finder for merge commits, as it wouldn't know about second parents.
The merge tests likely haven't caught this because they create simple histories, or the BCA finder
uses a different code path.

### 2. Update `add_parent` to enqueue ALL parents

**File**: `src/artifacts/log/rev_list.rs`

```rust
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

    // Load ALL parents and enqueue them
    for parent_oid in commit.parents() {
        self.load_commit(parent_oid)?;

        // Mark parents as uninteresting if this commit is uninteresting
        if is_uninteresting {
            self.mark_parents_uninteresting(parent_oid)?;
        }

        self.enqueue_commit(parent_oid)?;
    }

    if !is_uninteresting {
        self.simplify_commit(&commit)?;
    }

    Ok(())
}
```

### 3. Update `mark_parents_uninteresting` for correctness

**File**: `src/artifacts/log/rev_list.rs`

```rust
fn mark_parents_uninteresting(&mut self, oid: &ObjectId) -> anyhow::Result<()> {
    use std::collections::VecDeque;
    
    let mut queue = VecDeque::new();
    queue.push_back(oid.clone());

    while let Some(commit_oid) = queue.pop_front() {
        // Try to mark as uninteresting; if already marked, skip
        if !self
            .commits_flags
            .entry(commit_oid.clone())
            .or_default()
            .insert(LogTraversalCommitFlag::Uninteresting)
        {
            continue; // Already processed
        }

        // Load commit and enqueue ALL parents
        if let Some(commit) = self.commits_cache.get(&commit_oid) {
            for parent_oid in commit.parents() {
                self.load_commit(parent_oid)?;
                queue.push_back(parent_oid.clone());
            }
        }
    }

    Ok(())
}
```

## Testing Strategy

### Unit Tests (Not Architect's Responsibility)

Implementer and Tester agents will create:

1. **Simple merge commit traversal**
   - Create merge commit
   - Verify both parents visited
   - Check timestamp ordering

2. **Octopus merge traversal**
   - Create commit with 3+ parents
   - Verify all parents visited

3. **Diamond merge de-duplication**
   - Create diamond structure
   - Verify shared ancestor appears once

4. **First-parent traversal** (future)
   - Verify only first parent followed

### Integration Tests

Add to `tests/log/`:

1. `log_merge_commit_shows_all_parents.rs`
2. `log_octopus_merge_traversal.rs`
3. `log_diamond_merge_deduplication.rs`
4. `log_merge_with_range_exclusion.rs`

### Property Tests

Not required for this change (behavior well-defined).

## Roadmap Impact

This fixes a **semantic correctness bug** in current implementation.

Does NOT add new features. Makes existing `bit log` match Git semantics.

## Blocking Conditions

Implementation MUST NOT proceed until:

1. ✅ This design is reviewed and approved
2. ✅ Test plan is agreed upon
3. ✅ Implementer understands parent ordering semantics

## Algorithm Complexity

- **Time**: O(N log N) where N = number of commits
  - Each commit processed once: O(N)
  - Priority queue operations: O(log N) per commit
  - Total: O(N log N)

- **Space**: O(N)
  - Commit cache: O(N)
  - Flags map: O(N)
  - Priority queue: O(N) worst case

No change from current complexity.

## Compatibility

**Breaking change**: NO

Current behavior is wrong (only follows first parent by default).

Users with merge commits in their repos will see more complete output after this fix.

## Summary

### Current Problem
`bit log` only follows first parent of merge commits, missing entire branches.

### Root Cause
`add_parent()` calls `commit.parent()` (singular) instead of `commit.parents()` (plural).

### Solution
1. Add `Commit::parents()` accessor
2. **Fix `CommitCache::load_commit` to store all parents** (critical bug)
3. Update `add_parent()` to iterate all parents
4. Update `mark_parents_uninteresting()` to breadth-first search all parents

### Invariants Maintained
- DAG structure preserved
- Timestamp ordering maintained  
- No duplicate commits in output
- Parent order semantics preserved

### Complexity
No change: O(N log N) time, O(N) space

### Risk
Low. Clear fix with existing test infrastructure.

---

**Status**: Design complete, ready for implementation.

**Next Steps**:
1. Implementer: Add `parents()` method
2. Implementer: Update traversal logic
3. Tester: Add merge commit test cases
4. Verify against real Git behavior

