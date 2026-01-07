//! Git index file format
//!
//! The index (also called staging area or cache) stores information about the working tree.
//! It tracks which files should be included in the next commit.
//!
//! ## File Format (Version 2)
//!
//! ```text
//! Header (12 bytes):
//!   - Signature: "DIRC" (4 bytes)
//!   - Version: 2 (4 bytes)
//!   - Entry count (4 bytes)
//!
//! Entries (variable length):
//!   - Each entry padded to 8-byte alignment
//!   - Contains metadata and path
//!
//! Checksum (20 bytes):
//!   - SHA-1 hash of all preceding bytes
//! ```

pub mod checksum;
pub mod entry_mode;
pub mod index_entry;
pub mod index_header;

/// Size of SHA-1 checksum in bytes
pub const CHECKSUM_SIZE: usize = 20; // SHA1 produces a 20-byte hash

/// Size of index header in bytes
pub const HEADER_SIZE: usize = 12; // 4 bytes for marker, 4 for version, 4 for entries_count

/// Magic signature identifying index files
pub const SIGNATURE: &str = "DIRC"; // Signature for the index file

/// Index file format version
pub const VERSION: u32 = 2; // Version of the index file format
