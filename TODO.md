# TODO

Consolidated list of known improvements, inline code TODOs, and architectural enhancements.

---

## Architecture (High Priority)

These are structural issues that compound over time.

- [ ] **Drop tokio, use blocking mutex** — `tokio` full runtime is used solely to wrap a `Mutex`. Zero actual async I/O exists. Replace `tokio::sync::Mutex` with `std::sync::Mutex`, remove `async fn` from all command handlers, and drop `#[tokio::main]`. Affects `Cargo.toml`, `src/main.rs`, `src/areas/repository.rs`, all `src/commands/porcelain/*.rs`.
- [ ] **Introduce `thiserror` domain error types** — every function returns flat `anyhow::Result<T>`, making it impossible to distinguish "branch not found" from "permission denied" programmatically. Define `RefError`, `ObjectError`, `MergeError`, `IndexError`; keep `anyhow` only at the CLI boundary. (`src/main.rs` already has a TODO for this.)
- [ ] **Extract `CommitCache` from `Database`** — `CommitCache`/`SlimCommit` are graph-traversal structures with per-operation lifetime; they don't belong in the storage layer. Move them to `src/artifacts/merge/bca_finder.rs` (or a shared log module) so `Database` becomes a pure dumb store again. Affects `src/areas/database.rs`, `src/artifacts/merge/bca_finder.rs`.
- [ ] **Persist merge state** — conflicted merges leave no `.git/MERGE_HEAD` or `.git/MERGE_MSG`. If a user resolves conflicts and runs `bit commit`, the second parent is silently dropped. Add `read_merge_head`/`write_merge_head`/`clear_merge_head` to `src/areas/refs.rs`, write state on conflict in `src/artifacts/merge/resolution.rs`, and read it back in `src/commands/porcelain/commit.rs`.
- [ ] **Slim down `Repository` struct** — `reverse_refs` exists only for `log --decorate` but lives permanently on `Repository`. Move it to a local in `src/commands/porcelain/log.rs`. Also consolidate the three mutability strategies (`RefCell` × 3 + `tokio::Mutex`) into one consistent approach once tokio is removed.

---

## Inline Code TODOs

Sourced directly from `// TODO:` comments across the codebase.

### `src/main.rs`
- [ ] Improve error handling and messages using `thiserror` (line 31)
- [ ] Improve test harness using `snapbox` (line 32)

### `src/areas/database.rs`
- [ ] Implement packfiles for better performance and storage efficiency (line 53)
- [ ] Refactor to use async fs operations — or remove once tokio is dropped (line 54)

### `src/areas/workspace.rs`
- [ ] Refactor directory listing to use an iterator (line 99)
- [ ] Use flag options in workspace migration (line 233)

### `src/artifacts/status/status_info.rs`
- [ ] Use file change types separately for each area change (untracked, workspace, index) (line 13)

### `src/artifacts/merge/bca_finder.rs`
- [ ] Remove unnecessary cloning and optimize iterations in BCA result filtering (line 429)

### `src/artifacts/index/index_entry.rs`
- [ ] Restrict access to certain fields (line 40)
- [ ] Stop path traversal after reaching repository root (line 59)

### `src/artifacts/objects/tree.rs`
- [ ] Evaluate `ReadableTree`/`WritableTree` traits for better separation of concerns (line 83)
- [ ] Ensure directory names always end with `/` (line 155)

### `src/artifacts/objects/object.rs`
- [ ] Consider mutably borrowing `BufReader`/`BufWriter` for efficiency (line 28)
- [ ] Cache object serialization and ID to avoid recomputing on each call (line 56)

### `src/commands/porcelain/merge.rs`
- [ ] Pattern-match merge type (null, fast-forward, normal) and dispatch to dedicated handlers (line 8)

### `src/commands/porcelain/log.rs`
- [ ] Use a builder pattern for `LogOptions` (line 79)
- [ ] Use `&Path` instead of `PathBuf` in `LogOptions` (line 80)
- [ ] Define a `RepositoryWriter` trait to abstract over the writer (line 127)

### `src/commands/plumbing/ls_tree.rs`
- [ ] Add support for the `--recursive` flag (line 8)

---

## Roadmap (from README)

Unchecked items from the project roadmap.

### B. Index and snapshot construction
- [ ] Interactive staging (`add -p`)

### C. Commit graph and history traversal
- [ ] Extended ancestry/query expression parity with Git

### D. Workspace inspection and patching
- [ ] More advanced diff heuristics and rename/copy tracking

### E. Branching, checkout, merge
- [ ] More complete conflict resolution UX
- [ ] Rebase/cherry-pick workflows

### F. Remotes and packed storage
- [ ] Clone/fetch/push/pull protocols
- [ ] Packfiles and delta compression
- [ ] Reflog, hooks, and GC lifecycle tooling
- [ ] Transport handshake/capabilities (protocol v2 style)
- [ ] Object negotiation (`want`/`have`/ACK flows)
- [ ] Pack stream framing (pkt-line)
- [ ] Pack data model: base/delta representations (`OFS_DELTA`, `REF_DELTA`)
- [ ] Apply/delta pipeline: decode → resolve base chains → reconstruct → persist

---

## Testing Gaps

- [ ] Unit tests for individual components (currently only integration-level tests exist)
- [ ] Expand `proptest` usage beyond branch-name validation to: BCA invariants, diff endpoints, index ordering
- [ ] Add regression tests for `merge_state_persistence` scenarios (test file exists but merge state not yet implemented)
