pub mod branch_name;
pub mod checksum;
pub mod database_entry;
pub mod diff;
pub mod diff_target;
pub mod entry_mode;
pub mod file_change;
pub mod index_entry;
pub mod index_header;
pub mod inspector;
pub mod migration;
pub mod object;
pub mod object_id;
pub mod object_type;
pub mod revision;
pub mod status;
pub mod tree_diff;

pub const OBJECT_ID_LENGTH: usize = 40;
const CHECKSUM_SIZE: usize = 20; // SHA1 produces a 20-byte hash
pub const HEADER_SIZE: usize = 12; // 4 bytes for marker, 4 for version, 4 for entries_count
pub const SIGNATURE: &str = "DIRC"; // Signature for the index file
pub const VERSION: u32 = 2; // Version of the index file format
const INVALID_BRANCH_NAME_REGEX: &str =
    r"^\.|\/\.|\.\.|^\/|\/$|\.lock$|@\{|[\x00-\x20\*:\?\[\\~\^\x7f]";
const PARENT_REGEX: &str = r"^(.+)\^$";
const ANCESTOR_REGEX: &str = r"^(.+)\~(\d+)$";
const REF_ALIASES: phf::Map<&'static str, &'static str> = phf::phf_map! {
    "@" => "HEAD",
};
