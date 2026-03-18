//! Git command implementations
//!
//! This module contains all command implementations, organized into two categories
//! following Git's architecture:
//!
//! - `plumbing`: Low-level commands for direct object manipulation (hash-object, ls-tree)
//! - `porcelain`: User-facing commands for version control workflows (add, commit, log, etc.)
//!
//! Plumbing commands provide building blocks, while porcelain commands compose
//! them into higher-level operations.

pub mod plumbing;
pub mod porcelain;

/// Macro for debug logging that is enabled in test mode or with the debug_merge feature flag
///
/// # Usage
/// ```rust,ignore
/// debug_log!("Processing commit {}", commit_id);
/// ```
#[macro_export]
macro_rules! debug_log {
    ($($arg:tt)*) => {
        #[cfg(any(feature = "debug_merge"))]
        {
            eprintln!($($arg)*);
        }
    };
}
