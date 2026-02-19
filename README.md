# Bit - Just a lil' bit of Git in Rust

A Git implementation written in Rust, inspired by James Coglan’s book ["Building Your Own Git"](https://shop.jcoglan.com/building-git/). `bit` is intentionally educational, but it is built with production-style engineering discipline: explicit domain modeling, invariant-driven design, and a strong test suite.

## Overview

`bit` models Git as interactions between four core areas:

- **Object database** (`.git/objects`): immutable content-addressed objects.
- **Index** (`.git/index`): staging-area metadata and next-tree blueprint.
- **References** (`.git/HEAD`, `refs/*`): symbolic and direct pointers into history.
- **Workspace**: mutable files checked out for editing.

Most command behavior can be reasoned about as transitions across these areas.

## Implemented Commands

- ✅ `bit init`
- ✅ `bit hash-object`
- ✅ `bit ls-tree`
- ✅ `bit add`
- ✅ `bit commit`
- ✅ `bit status`
- ✅ `bit diff`
- ✅ `bit branch` (create/list/delete)
- ✅ `bit checkout`
- ✅ `bit log`
- ✅ `bit merge` (multi-scenario DAG merges, conflict-aware behavior)

## Domain Models and Invariants

### 1) Object model
- Supports Git object categories needed by current commands (blob/tree/commit flows).
- Serialization follows Git format: `<type> <size>\0<content>`.
- Object IDs are SHA-1 of serialized object bytes.
- Objects are immutable once written.

### 2) Index model
- Entries are deterministically ordered.
- Header entry count reflects serialized entries.
- Index checksum verifies integrity.
- Parent/child path conflicts are normalized when replacing file/dir shapes.
- Index read/write uses locking semantics to maintain consistency under concurrent operations.

### 3) Revision + refs model
- Branch and revision parsing supports common forms (`ref`, `^`, `~n`, aliases).
- Branch name validation follows Git-like constraints and explicit parser rules.
- HEAD and refs are managed as first-class repository state.

### 4) Merge and graph model
- Merge behavior is validated on multiple non-trivial commit DAGs.
- Best common ancestor scenarios and branching edge-cases are covered by integration tests.


### 5) Diff and patch model
- Diff endpoints are explicit (`workspace↔index`, `index↔HEAD`, `rev↔rev`).
- File-level status (`A`,`D`,`M`, mode-only changes) is deterministic.
- Patch hunks should remain stable in ordering and context output semantics.

### 6) Log traversal model
- Traversal honors included and excluded revision expressions.
- Commit output ordering should be deterministic, including tie-breaking for identical timestamps.
- Decorations are a read-only projection of current refs over commits.

### 7) Checkout migration model
- Checkout is a controlled migration between revisions across refs, index, and workspace.
- Safety checks should prevent silent clobbering of local/staged work.
- Success means workspace/index are synchronized to target tree semantics.

### 8) Core algorithms and data structures
- **Merkle DAG** for commit/history integrity and reachability.
- **Tree/index path hierarchy (trie-like)** for parent/child path operations.
- **Revision AST** to represent references, parent (`^`), ancestor (`~n`), ranges, and exclusions.
- **Myers-style diff baseline** for minimal edit script patch construction (future enhancements may refine heuristics).
- **Binary codec discipline** for future wire protocol framing and pack parsing.

## Rust Engineering Practices

This project emphasizes:

- idiomatic ownership and borrowing,
- explicit `Result`-based error handling,
- deterministic filesystem interaction,
- lock-aware repository mutations,
- incremental changes with strong test coverage.

## Architecture

```text
src/
├── main.rs              # CLI interface and command routing
├── commands/            # Command implementations
│   ├── plumbing/        # Low-level Git commands
│   └── porcelain/       # User-facing commands
├── areas/               # Repository areas: database/index/refs/workspace
└── artifacts/           # Domain objects and algorithms (objects, diff, merge, log, status)
```

## How to Run Locally

### Prerequisites

- Rust 1.93 or later
- Cargo

### Build

```bash
cargo build --release
```

Binary path:

```bash
target/release/bit
```

## Usage

```bash
# initialize repository
bit init [path]

# write or hash objects
bit hash-object [-w] <file>
bit ls-tree [-r] <tree-sha>

# staging + commits
bit add <path>...
bit commit -m "message"

# inspect state
bit status [--porcelain]
bit diff [--cached] [--name-status] [--diff-filter=ADMR] [old] [new]
bit log [targets...] [-- --paths] [--oneline] [--abbrev-commit] [--decorate=<none|short|full>] [--patch]

# branch / checkout / merge
bit branch create <name> [source]
bit branch list [-v]
bit branch delete <name>... [-f]
bit checkout <target-revision>
bit merge <target-revision> -m "merge message"
```

## Testing

The test suite combines unit, property-based, and integration tests to validate Git semantics.

### Environment setup

```bash
mkdir -p ../playground
```

### Run tests

```bash
cargo test
cargo test -- --nocapture
```

Targeted examples:

```bash
cargo test add
cargo test commit
cargo test diff
cargo test log
cargo test merge
```

## Testing Strategy (TDD + Coglan-style semantics)

### TDD loop
1. Write a failing test from behavior/spec.
2. Implement minimum code to pass.
3. Refactor while preserving green tests.

### Unit tests
Use for pure logic and parsers:
- revision syntax,
- branch-name validation,
- object payload formatting/parsing,
- small deterministic transforms.

### Property tests (`proptest`)
Use for invariant-heavy behavior:
- valid/invalid revision and branch grammar,
- parser determinism,
- boundary input spaces.

Store generated failing cases in `proptest-regressions/` when applicable.

### Integration tests
Use command-level scenarios under `tests/`:
- compare command output and state transitions,
- cover merge graph topologies,
- validate index/workspace/ref interactions,
- compare against `git` where practical.


### Merge / diff / log invariant tests to prioritize
- **Merge**: fast-forward eligibility, criss-cross BCAs, multi-parent lineage ordering, conflict marker persistence.
- **Diff**: endpoint correctness, mode-only changes, multi-file hunk stability, deterministic status ordering.
- **Log**: include/exclude revision expressions, path-filtered history, stable ordering with timestamp ties, decoration rendering variants.

### Remote + pack tests (future)
- pkt-line codec round-trips and malformed-frame rejection.
- negotiation state-machine tests for `want/have/ack` flows.
- pack decoding tests for full objects and chained deltas.
- object-integrity checks after unpack + write.

## Roadmap aligned with *Building Git from Scratch*

> The roadmap is organized by domain themes that mirror the book’s progression from low-level object mechanics to higher-level collaboration/storage concerns.

### A. Object storage and plumbing
- [x] Initialize repository structure
- [x] Hash/write loose objects
- [x] Read/tree-walk object structures
- [ ] Additional plumbing introspection and validation commands

### B. Index and snapshot construction
- [x] Stage files and directory trees
- [x] Handle file/dir replacement conflicts
- [x] Persist and validate index metadata/checksum
- [ ] Interactive staging (`add -p`)

### C. Commit graph and history traversal
- [x] Write commits and parent relationships
- [x] Traverse history for `log`
- [x] Revision expressions and branch-based targeting
- [ ] Extended ancestry/query expressions parity

### D. Workspace inspection and patching
- [x] `status` for staged/unstaged/untracked states
- [x] `diff` for workspace/index/commit comparisons
- [x] Patch-oriented log output
- [ ] More advanced diff heuristics and rename/copy tracking

### E. Branching, checkout, merge
- [x] Branch create/list/delete
- [x] Checkout with ref/symbolic-ref behavior
- [x] Merge for complex DAG scenarios (including multi-BCA patterns)
- [ ] More complete conflict resolution UX
- [ ] Rebase/cherry-pick workflows

### F. Remotes and packed storage
- [ ] Clone/fetch/push/pull protocols
- [ ] Packfiles and delta compression
- [ ] Reflog, hooks, and GC lifecycle tooling


#### Remote flow and pack protocol (expanded)
- **Transport handshake/capabilities**: model protocol v2 style negotiation (capabilities, command selection, shallow/filter options where relevant).
- **Object negotiation**: exchange `want`/`have` sets and ACK states to minimize transfer.
- **Pack stream framing**: use pkt-line length-prefixed binary records with flush/delim packets.
- **Pack data model**: support object entry headers, base/delta representations (`OFS_DELTA`, `REF_DELTA`), and integrity verification at stream/repo boundaries.
- **Apply/delta pipeline**: decode + resolve base chains + reconstruct canonical objects before persistence.
- **Safety invariants**: never accept malformed framing/checksums silently; keep object graph/hash checks explicit.

## Known Gaps

- No remote/network protocol support yet.
- Packed object storage and delta compression are not implemented.
- History-rewriting workflows (rebase/cherry-pick) are pending.
- Merge UX can be improved for richer conflict presentation/resolution workflows.

## Contributing

1. Start from a failing test.
2. Implement minimally.
3. Keep changes small and focused.
4. Ensure formatting and tests pass.
5. Update README/instructions if behavior changes.

## Acknowledgments

- James Coglan for *Building Your Own Git*.
- The Git project for behavioral reference.
- The Rust ecosystem for excellent tooling.
