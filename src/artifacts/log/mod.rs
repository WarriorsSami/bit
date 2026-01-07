//! Commit history traversal for git log
//!
//! This module implements the core `git log` functionality:
//!
//! - `rev_list`: Revision list traversal with range expressions
//! - `path_filter`: Efficient path filtering using trie data structure
//!
//! ## Algorithm
//!
//! The log traversal uses a priority queue ordered by commit timestamp,
//! supporting:
//! - Range expressions (commit1..commit2)
//! - Excluded revisions (^commit)
//! - Path filtering (show only commits affecting specific files)
//! - Proper handling of merge commits and complex histories

pub mod path_filter;
pub mod rev_list;
