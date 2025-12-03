pub mod branch_name;
pub mod revision;

pub const INVALID_BRANCH_NAME_REGEX: &str =
    r"^\.|\/\.|\.\.|^\/|\/$|\.lock$|@\{|[\x00-\x20\*:\?\[\\~\^\x7f]";
pub const PARENT_REGEX: &str = r"^(.+)\^$";
pub const ANCESTOR_REGEX: &str = r"^(.+)\~(\d+)$";
pub const REF_ALIASES: phf::Map<&'static str, &'static str> = phf::phf_map! {
    "@" => "HEAD",
};
