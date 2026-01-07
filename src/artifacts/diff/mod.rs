//! Diff algorithms and tree comparison
//!
//! This module implements various diffing algorithms:
//!
//! - `diff_algorithm`: Myers' diff for line-by-line comparison
//! - `diff_target`: Abstraction over diff sources (workspace, index, commits)
//! - `tree_diff`: Tree-level diffing for detecting file changes
//!
//! The diff implementation supports both tree-level (which files changed)
//! and content-level (what changed within files) comparison.

pub mod diff_algorithm;
pub mod diff_target;
pub mod tree_diff;
