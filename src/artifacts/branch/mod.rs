//! Branch and revision management
//!
//! This module handles:
//! - Branch name validation and parsing
//! - Revision specification parsing (refs, OIDs, parent notation, etc.)
//! - Symbolic reference resolution
//!
//! ## Revision Syntax
//!
//! Supports Git-compatible revision syntax:
//! - Branch names: `main`, `feature/new-feature`
//! - Aliases: `@` → `HEAD`
//! - Parent notation: `HEAD^`, `main^`
//! - Ancestor notation: `HEAD~3`, `main~5`
//! - Object IDs: Full (40 chars) or abbreviated (4-40 chars)

pub mod branch_name;
pub mod revision;

/// Regex pattern for invalid characters in branch names
pub const INVALID_BRANCH_NAME_REGEX: &str =
    r"^\.|\/\.|\.\.|^\/|\/$|\.lock$|@\{|[\x00-\x20\*:\?\[\\~\^\x7f]";

/// Regex pattern for parent notation (e.g., "HEAD^")
pub const PARENT_REGEX: &str = r"^(.+)\^$";

/// Regex pattern for ancestor notation (e.g., "HEAD~3")
pub const ANCESTOR_REGEX: &str = r"^(.+)\~(\d+)$";

/// Map of revision aliases to their expansions
pub const REF_ALIASES: phf::Map<&'static str, &'static str> = phf::phf_map! {
    "@" => "HEAD",
};
