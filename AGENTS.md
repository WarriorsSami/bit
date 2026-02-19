# AGENTS.md — Bit AI Agent Operating Guide

This file defines how AI coding agents should operate in this repository.

## Objective

Deliver correct, incremental improvements to `bit`, a Rust implementation of Git internals inspired by James Coglan’s *Building Git from Scratch*, while preserving domain invariants and test rigor.

## Core principles

1. **Correctness over cleverness**: match Git semantics before optimizing.
2. **Small diffs**: isolate concerns, avoid broad rewrites.
3. **Tests first**: TDD for all behavior changes.
4. **Explainability**: code should be readable and reviewable.

## Agent roles and responsibilities

### 1) Architect
Use when designing or changing behavior across modules.
- Define domain model impact first (objects/index/refs/workspace).
- Enumerate invariants that must remain true.
- Propose migration-safe rollout for CLI-visible behavior.

### 2) Implementer
Use when writing/modifying Rust code.
- Implement smallest change that satisfies the test/spec.
- Keep ownership/error handling idiomatic (`Result`, `match`, domain types).
- Respect existing module boundaries and naming conventions.

### 3) Tester
Use for verification and regression prevention.
- Add failing tests first for new behavior/bugs.
- Cover happy path + edge cases + failure paths.
- Prefer integration tests for command workflows; unit/proptest for parser and invariant logic.

### 4) Refiner
Use for quality hardening.
- Run fmt/lint/tests.
- Improve diagnostics and docs.
- Ensure README and instruction docs stay aligned with implementation.

## Required workflow

For any non-trivial change:
1. State affected domain areas (objects/index/refs/workspace).
2. Add or update tests first.
3. Implement minimal fix/feature.
4. Run checks (`cargo fmt`, `cargo test`, additional targeted checks).
5. Update docs (`README.md`) if behavior/status changed.

## Domain model + invariants

### Object database
- Objects are immutable and content-addressed.
- Serialization format `<type> <size>\0<content>` must be preserved.
- Hash is computed from serialized bytes exactly.

### Index
- Entry ordering deterministic.
- Entry count and checksum consistent with bytes written.
- Parent/child path conflicts are resolved consistently.
- File locking required for safe concurrent access.

### Refs / revisions
- HEAD and branch refs must stay internally consistent.
- Revision parsing must remain deterministic and reject invalid forms.
- Branch naming rules must follow current parser constraints.

### Workspace transitions
- Commands changing workspace/index/refs must be explicit and test-covered.
- Conflict scenarios must preserve data safety and clear user feedback.

### Merge / diff / log invariants
- Merge:
  - detect fast-forward eligibility correctly,
  - compute best common ancestor(s) over the commit DAG,
  - preserve parent ordering for merge commits,
  - never lose conflicting user content silently.
- Diff:
  - define comparison endpoints unambiguously,
  - preserve deterministic file/hunk ordering,
  - maintain consistent status classification (A/D/M/mode changes).
- Log:
  - deterministic traversal with include/exclude revision expressions,
  - stable output ordering for timestamp ties,
  - decoration must be a read-only projection of refs.

### Checkout migration invariants
- Checkout is a coordinated migration of refs + index + workspace.
- Reject or guard transitions that would clobber local changes.
- Successful checkout leaves index/workspace matching target tree semantics.

### Remote + pack protocol invariants (roadmap)
- Protocol framing must be binary-safe and length-delimited (pkt-line discipline).
- Capability negotiation must be explicit and testable.
- Pack ingestion/emission must preserve object graph integrity and hash identity.

## Testing policy

### Unit tests
Use for:
- parsing, formatting, helper algorithms.
- deterministic transformations.

### Property tests (`proptest`)
Use for:
- grammar and parser invariants.
- determinism/idempotence properties.
- boundary-heavy input spaces.

Guidelines:
- Prefer constructive strategies.
- Keep cases meaningful and bounded.
- Commit regression seeds under `proptest-regressions/` when generated.

### Integration tests
Use command-level black-box tests under `tests/`.
- Validate exit code, output, and repo state.
- Model realistic DAGs for branch/log/merge behavior.
- Compare with `git` behavior when practical.
- Add migration tests for checkout edge-cases (staged/unstaged conflicts).
- Add deterministic-order tests for log/diff output.

### Algorithmic focus for future test suites
- Merkle DAG reachability and best-common-ancestor correctness.
- Myers-style diff behavior and hunk stability.
- Revision-expression parser coverage (`^`, `~n`, ranges, exclusions).
- Remote negotiation and pack codec conformance once implemented.

## Rust best practices for this repo

- Use `anyhow` + contextual errors for fallible operations.
- Avoid `unsafe` unless absolutely necessary and documented.
- Keep mutable state local; isolate side effects.
- Use filesystem operations conservatively with robust error checks.
- Treat concurrency as correctness-sensitive (locking + deterministic tests).

## Anti-patterns (avoid)

- Large untested refactors.
- Hidden CLI output changes without test updates.
- Swallowing I/O errors in repository-critical paths.
- Over-general abstractions with no current use.
- Introducing dependencies without clear justification.

## README alignment requirement

If implementation status changes, agents must:
- update feature status/limitations,
- update roadmap checkpoints where appropriate,
- keep usage/testing instructions accurate.

## Suggested Coglan-aligned roadmap themes

When proposing roadmap updates, organize by these themes:
1. Objects and storage internals.
2. Index and staging semantics.
3. Commit graph and history traversal.
4. Branching/revision expressions.
5. Diff/merge algorithms and conflict handling.
6. Remote protocols and packed storage.

## Chapter-to-implementation guidance

When expanding roadmap details or writing specs, anchor work items to these Coglan-style topics:
1. Object database + Merkle identity.
2. Trees/index as path hierarchy (trie-like organization).
3. Revision parsing and graph traversal.
4. Diff algorithm choices (Myers baseline, patch formatting semantics).
5. Merge base selection + three-way merge/migration policy.
6. Wire protocol binary codec + packfile negotiation/compression.

## Pre-PR checklist

- [ ] Domain invariants identified.
- [ ] Tests added/updated first.
- [ ] Implementation minimal and focused.
- [ ] `cargo fmt` and `cargo test` run.
- [ ] README/docs updated for behavior changes.
- [ ] No unrelated file churn.
