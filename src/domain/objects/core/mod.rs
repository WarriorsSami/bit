pub mod checksum;
pub mod entry_mode;
pub mod index_entry;
pub mod index_header;
pub mod object;
pub mod object_id;
pub mod object_type;

pub const CHECKSUM_SIZE: usize = 20; // SHA1 produces a 20-byte hash
pub const HEADER_SIZE: usize = 12; // 4 bytes for marker, 4 for version, 4 for entries_count
pub const SIGNATURE: &str = "DIRC"; // Signature for the index file
pub const VERSION: u32 = 2; // Version of the index file format
