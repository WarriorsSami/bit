//! Plumbing commands (low-level Git operations)
//!
//! Plumbing commands provide direct access to Git's internal data structures
//! and operations. They're primarily used for scripting and as building blocks
//! for porcelain commands.
//!
//! ## Commands
//!
//! - `hash-object`: Compute object ID and optionally store in database
//! - `ls-tree`: List contents of a tree object

pub mod hash_object;
pub mod ls_tree;
mod write_commit;
