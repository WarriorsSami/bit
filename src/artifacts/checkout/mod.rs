//! Checkout operations and conflict handling
//!
//! This module handles switching between commits by:
//! - Computing differences between current and target states
//! - Detecting conflicts with local modifications
//! - Planning and executing file system changes
//! - Updating the index to match the target commit
//!
//! Checkout operations are designed to be safe, detecting all conflicts
//! before making any changes to the working directory.

pub mod conflict;
pub mod migration;
