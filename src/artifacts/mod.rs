//! Git data structures and algorithms
//!
//! This module contains the core Git types and algorithms:
//!
//! - `branch`: Branch names and revision parsing
//! - `checkout`: Checkout operations and conflict detection
//! - `core`: Shared utilities (pager wrapper, etc.)
//! - `database`: Database entry types
//! - `diff`: Tree diffing algorithms (Myers' diff)
//! - `index`: Index/staging area data structures
//! - `log`: Commit history traversal and filtering
//! - `objects`: Git object types (blob, tree, commit)
//! - `status`: Working tree status inspection
//! - `merge`: Merge algorithms and conflict resolution

pub mod branch;
pub mod checkout;
pub mod core;
pub mod database;
pub mod diff;
pub mod index;
pub mod log;
pub mod merge;
pub mod objects;
pub mod status;
