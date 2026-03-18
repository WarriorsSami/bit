# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Build and Test Commands

```bash
# Build
cargo build --release

# Run all tests (requires playground directory)
mkdir -p ../playground
cargo test

# Run tests with output visible
cargo test -- --nocapture

# Run a specific test module
cargo test merge
cargo test log
cargo test diff

# Run a single test by name
cargo test merge_fast_forward

# Enable merge debug logging
cargo test --features debug_merge
```

## Architecture

`bit` is a Git implementation in Rust. Repository state is modeled as four areas:

- **`areas/database`** — Object storage (`.git/objects`): blob/tree/commit, SHA-1 addressed, zlib compressed
- **`areas/index`** — Staging area (`.git/index`): lock-protected read/write, deterministic entry ordering, checksum verified
- **`areas/refs`** — References (`.git/HEAD`, `refs/heads/*`): symbolic refs, branch pointers, current HEAD tracking
- **`areas/workspace`** — Working directory: file reads, stat data, executable detection

`areas/repository.rs` is the **facade**: `Repository` composes the four areas and all command implementations live as `impl Repository` methods spread across modules in `commands/porcelain/` and `commands/plumbing/`.

`artifacts/` holds domain objects and algorithms:
- `objects/` — `ObjectId`, blob/tree/commit types, serialization
- `merge/` — BCA finder (best common ancestor via DAG traversal), merge inputs, resolution
- `diff/` — Myers-style diff, hunk generation
- `log/` — Commit traversal, revision targets/exclusions
- `branch/` — `BranchName`, `SymRefName`, validation
- `core/` — `PagerWriter`, shared utilities

CLI entry point is `main.rs` using `clap` derive macros. Commands dispatch to `Repository` methods; pager wrapping happens at the top level for `log`, `diff`, and `branch list`.

## Commit Message Format

All commits must follow **Conventional Commits**:

```
<type>(<scope>): <description>
```

Types: `feat`, `fix`, `test`, `refactor`, `docs`, `style`, `perf`, `build`, `ci`, `chore`

Common scopes: `(merge)`, `(index)`, `(refs)`, `(diff)`, `(log)`, `(checkout)`, `(objects)`, `(status)`, `(cli)`, `(tests)`

Examples: `feat(merge): add three-way merge algorithm`, `fix(index): prevent file/dir path conflicts`

## Integration Test Conventions

Tests live under `tests/` organized by command (e.g. `tests/merge/`, `tests/log/`). Each module is declared in `tests/<command>/mod.rs`.

Test helpers are in `tests/common/`:
- `common::redirect_temp_dir()` — must be called in integration tests; redirects temp files to `../playground`
- `common::command` — helpers to run `bit` and `git` commands and capture output
- `assert_index_eq!` — compares index binary contents with hexdump on failure

The `../playground` directory must exist before running tests. Tests create and manipulate real `.git` repositories in temp directories within it.

## Key Domain Rules

- Objects are **immutable** once written; never mutate an object, produce a new one
- Index read/write uses **lock-file discipline** — always release the lock after write
- Parent ordering in merge commits is **semantically significant**: first parent = current branch lineage
- Revision parsing handles `ref`, `^`, `~n`, `@` (HEAD alias), ranges, and exclusions — always use the `Revision` type, not raw strings
- Three-way merge inputs are always `(base, ours, theirs)` in that order
- Fast-forward is only valid when target is a descendant of HEAD

## Feature Flags

- `debug_merge` — enables `debug_log!` macro output in BCA finder; useful when debugging merge DAG traversal
