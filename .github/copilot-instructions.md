# Copilot Instructions for Bit (Rust Git implementation)

This document guides GitHub Copilot suggestions for the `bit` codebase.

## Mission

Treat `bit` as an educational, correctness-first Git implementation inspired by James Coglan’s *Building Git from Scratch*. Prioritize:

1. Faithfulness to Git’s data model and invariants.
2. Readable, idiomatic Rust.
3. Test-driven delivery (unit + integration + property tests).
4. Small, reviewable changes.

## Domain Model (must preserve)

### Core repository model
- Repository state is the interaction of:
  - **Object database** (`.git/objects`): immutable blob/tree/commit objects.
  - **Index** (`.git/index`): staging area snapshot-in-progress.
  - **Refs** (`.git/HEAD`, `refs/heads/*`): names pointing to commits.
  - **Workspace**: mutable working files.
- Commands should make state transitions explicit across these four areas.

### Git objects and serialization
- Git object wire format is `<type> <size>\0<content>`.
- Object IDs are SHA-1 of the full serialized representation.
- Objects are immutable and content-addressed: avoid mutation-based logic.

### Revision/refs semantics
- Accept and parse revision syntax deterministically (e.g. refs, parent `^`, ancestor `~n`, aliases like `@` -> `HEAD`).
- Keep branch/ref validation compatible with existing parser behavior.

### Index invariants
- Index entries remain deterministically ordered.
- Header/count/checksum must match serialized entries.
- File/dir replacement conflicts must be resolved consistently (parent file vs child paths).
- Read/write must respect lock discipline.

### History/Merkle-DAG invariants
- The commit graph is a Merkle DAG: each commit hash commits to its tree and parent hashes.
- Any rewrite to commit metadata/tree/parent list must produce a new object ID.
- Parent ordering in merge commits is semantically meaningful (first parent is current branch lineage).
- Reachability queries (`log`, merge-base search) must not mutate graph state.

### Merge invariants
- Fast-forward is valid only when target is descendant-compatible with current HEAD lineage.
- Non-fast-forward merge requires best common ancestor (BCA) discovery over the DAG.
- Three-way merge input must be `(base, ours, theirs)` in a consistent order.
- Conflict markers and index/workspace transitions must never silently drop user content.

### Diff invariants
- Diff operations must clearly define endpoints: workspace vs index, index vs HEAD, or revision vs revision.
- Path status classification (`A`,`D`,`M`, mode changes) must be deterministic.
- Hunk generation should preserve stable ordering and context semantics.

### Log invariants
- Traversal should respect revision targets and exclusions deterministically.
- Output order must be stable for identical timestamps (tie-break deterministically).
- Decorations (`--decorate`) must reflect refs at display time without mutating refs.

### Checkout migration invariants
- Migration between revisions is a state transition across refs + index + workspace.
- Uncommitted local modifications that would be lost must be detected and surfaced.
- Index/workspace must converge to target tree state after a successful checkout.

### Remote and pack protocol boundaries
- Current implementation is local-first; remote protocol features are roadmap items.
- Future remote flow should model Git upload-pack/receive-pack negotiation explicitly.
- Pack protocol implementation must preserve object identity/integrity and sideband framing rules.

## Rust implementation rules

### Error handling
- Use `Result<T, anyhow::Error>` style already used in the repo.
- Add context to errors that cross module boundaries.
- Avoid panics in production paths; panic in tests is acceptable.

### Types and ownership
- Prefer expressive domain types (`ObjectId`, `Revision`, `BranchName`) over raw strings.
- Keep borrowing/local ownership simple; avoid over-abstracting lifetimes.
- Favor `match` over chained conditionals for state transitions.

### Filesystem interaction
- Be explicit about paths, normalization assumptions, and path safety.
- Use atomic-ish patterns where possible (lock + write + checksum verification).
- Never silently swallow I/O errors that indicate repository corruption.

### Concurrency patterns
- Concurrency is correctness-sensitive around index/repo state.
- Preserve locking guarantees for concurrent command execution.
- Introduce async/concurrency only when behavior is test-covered and deterministic.

### Algorithms and data structures to prefer
- **Merkle DAG**: commit history + reachability, BCA discovery, topological traversal.
- **Trie-like path modeling**: tree/index path hierarchy and parent-child conflict handling.
- **Myers-style diff model**: shortest edit script assumptions for textual patches.
- **Binary codec discipline**: explicit byte-order, length framing, and checksum boundaries.
- **Revision parser AST**: represent expressions (`ref`, `^`, `~n`, ranges, exclusions) as typed enums.

### CLI behavior
- Keep output stable where tests expect exact formatting.
- Prefer additive CLI changes and backward compatibility.

## TDD workflow (required)

For behavior changes:
1. Write/adjust a failing test first.
2. Implement minimal code to pass.
3. Refactor while keeping tests green.

For bug fixes:
1. Add a regression test reproducing the bug.
2. Fix with minimal semantic diff.
3. Keep fix covered by integration/unit tests.

## Test authoring guide

### Unit tests
Use for:
- Parsing/formatting (`Revision`, branch names, object payload parsing).
- Deterministic algorithmic helpers.

Pattern:
- clear arrange/act/assert structure.
- assert both happy path and edge/failure path.

### Property tests (`proptest`)
Use for invariants and parser stability:
- valid vs invalid branch names.
- revision round-trip/determinism.
- bounded combinatorics for suffix operators (`^`, `~n`).

Rules:
- build constrained strategies reflecting Git rules.
- use `prop_filter` sparingly; prefer constructive strategies.
- when failures occur, commit resulting regression cases in `proptest-regressions/`.

### Integration tests
Use command-level tests under `tests/`:
- model user workflows across index/workspace/refs/object db.
- compare with Git behavior when practical.
- cover merge DAG scenarios and conflict behavior explicitly.
- include checkout migration scenarios where local/staged states interact with target revisions.
- add log/diff coverage for range expressions, exclusions, and stable ordering guarantees.

### Algorithm-focused test vectors
- **Merge/BCA**: diamond, criss-cross, octopus, and identical-head merges.
- **Diff**: mode-only changes, binary file handling policy, multi-hunk updates.
- **Log**: mixed include/exclude revisions, path-limited history, decoration formatting.
- **Remote/pack (future)**: pkt-line framing, capability negotiation, object negotiation loops, thin-pack fixes.

Recommended Given/When/Then shape in test comments:
- **Given** repository graph/files.
- **When** command is run.
- **Then** stdout/stderr, exit code, repo state and file contents.

## Copilot generation constraints

When proposing code:
- Do not invent unsupported Git semantics silently.
- Do not add `unsafe` unless unavoidable and justified.
- Do not introduce unrelated dependencies without clear need.
- Keep functions small and focused; prefer pure helpers for transformations.
- Preserve existing naming/style/module layout unless refactoring is part of the task.

When proposing tests:
- Prefer tests that lock in domain invariants.
- Include edge cases from Git semantics (empty repos, ambiguous refs, path conflicts, merge base edge cases).
- For concurrency-sensitive code, include race-resistant reproducible tests.

## Review checklist for generated code

Before accepting Copilot output, verify:
- [ ] Correctness against Git model (objects, refs, index semantics).
- [ ] Error handling quality and context.
- [ ] No hidden behavior changes in CLI output.
- [ ] New/updated tests cover behavior and edge cases.
- [ ] `cargo test` and formatting/lint checks pass.

## Documentation update policy

When behavior changes:
- update `README.md` implementation status.
- update usage examples if CLI flags/commands changed.
- update roadmap checkboxes only if functionality is genuinely shipped.
