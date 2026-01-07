//! Git object types and operations
//!
//! Git stores all content as objects identified by SHA-1 hashes. There are four main types:
//!
//! - **Blob**: File content (raw bytes)
//! - **Tree**: Directory listing (names, modes, and object IDs)
//! - **Commit**: Snapshot with metadata (author, message, parent commits, tree)
//! - **Tag**: Annotated reference to another object
//!
//! All objects implement serialization/deserialization for the Git object format:
//! `<type> <size>\0<content>`

pub mod blob;
pub mod commit;
pub mod object;
pub mod object_id;
pub mod object_type;
pub mod tree;

/// Length of a SHA-1 hash in hexadecimal format
pub const OBJECT_ID_LENGTH: usize = 40;
