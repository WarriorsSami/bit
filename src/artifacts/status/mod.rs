//! Working tree status inspection
//!
//! This module provides functionality for analyzing the state of the working directory
//! by comparing it against the index and HEAD commit.
//!
//! ## Components
//!
//! - `file_change`: Enum types for categorizing changes
//! - `inspector`: Core logic for detecting changes
//! - `status_info`: Status information aggregation and display

pub mod file_change;
pub mod inspector;
pub mod status_info;
