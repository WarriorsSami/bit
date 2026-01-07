//! Core repository components
//!
//! This module contains the fundamental building blocks of a Git repository:
//!
//! - `database`: Object database for storing blobs, trees, and commits
//! - `index`: Staging area (index/cache) for tracking file changes
//! - `refs`: Reference management (branches, HEAD, tags)
//! - `repository`: High-level repository operations and coordination
//! - `workspace`: Working directory file system operations

pub(crate) mod database;
pub(crate) mod index;
pub(crate) mod refs;
pub mod repository;
pub(crate) mod workspace;
