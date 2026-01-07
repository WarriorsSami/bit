//! Porcelain commands (user-facing Git operations)
//!
//! Porcelain commands provide the high-level user interface for version control.
//! They compose plumbing commands and internal operations into workflows that
//! match typical Git usage patterns.
//!
//! ## Commands
//!
//! - `init`: Initialize a new repository
//! - `add`: Stage files for commit
//! - `commit`: Create a new commit
//! - `status`: Show working tree status
//! - `diff`: Show changes between commits/trees
//! - `log`: Show commit history
//! - `branch`: Create, list, or delete branches
//! - `checkout`: Switch branches or restore files

pub mod add;
pub mod branch;
pub mod checkout;
pub mod commit;
pub mod diff;
pub mod init;
pub mod log;
pub mod status;
