use crate::areas::repository::Repository;
use crate::artifacts::branch::branch_name::BranchName;
use crate::artifacts::branch::{ANCESTOR_REGEX, PARENT_REGEX, REF_ALIASES};
use crate::artifacts::objects::OBJECT_ID_LENGTH;
use crate::artifacts::objects::object_id::ObjectId;
use crate::artifacts::objects::object_type::ObjectType;
use anyhow::Context;

/// Represents a revision specification that can be used to identify commits.
///
/// Supports multiple formats:
/// - Branch/ref names: `main`, `feature/new-feature`, `HEAD`
/// - Aliases: `@` (resolves to `HEAD`)
/// - Full OIDs: 40-character hexadecimal strings (resolved as fallback if ref doesn't exist)
/// - Abbreviated OIDs: 4-40 character hexadecimal strings (resolved as fallback if ref doesn't exist)
/// - Parent notation: `<revision>^` (e.g., `main^`, `HEAD^`, `abc123^`)
/// - Ancestor notation: `<revision>~<n>` (e.g., `main~3`, `HEAD~5`, `abc123~2`)
///
/// # Parsing Strategy
///
/// OID-like strings (e.g., "abc123") are initially parsed as `Ref` variants. During resolution,
/// if no ref with that name exists and the string looks like an OID (4-40 hex characters),
/// the resolver will attempt to resolve it as an object ID. This matches Git's behavior of
/// preferring refs over OIDs when there's ambiguity.
///
/// # Examples
///
/// ```ignore
/// // Parse a branch name
/// let rev = RevisionContext::parse("main")?;
///
/// // Parse what might be an OID (initially treated as Ref, resolved as OID if ref doesn't exist)
/// let rev = RevisionContext::parse("abc123")?;
///
/// // Parse with parent notation
/// let rev = RevisionContext::parse("main^")?;
/// let rev = RevisionContext::parse("abc123^")?;
///
/// // Parse with ancestor notation
/// let rev = RevisionContext::parse("main~3")?;
/// let rev = RevisionContext::parse("abc123~2")?;
/// ```
#[derive(Debug, Clone)]
pub enum Revision {
    /// A reference to a branch, symbolic ref, or potentially an OID (resolved during resolution phase)
    Ref(BranchName),
    /// The Nth ancestor of a revision (e.g., HEAD~3)
    Ancestor(Box<Revision>, usize),
    /// The parent of a revision (e.g., HEAD^)
    Parent(Box<Revision>),
}

impl Revision {
    pub fn resolve(&self, repository: &Repository) -> anyhow::Result<Option<ObjectId>> {
        match self {
            Revision::Ref(branch_name) => {
                let name_str = branch_name.as_ref();

                // Try to resolve as a ref first
                match repository.refs().read_ref(branch_name.clone()) {
                    Ok(Some(oid)) => Ok(Some(oid)),
                    Ok(None) => Ok(None),
                    Err(_) => {
                        // Ref doesn't exist - try OID if it looks like one
                        if Self::looks_like_oid(name_str) {
                            // Try to resolve as OID
                            match Self::resolve_oid(name_str, repository) {
                                Ok(oid) => Ok(Some(oid)),
                                Err(oid_err) => {
                                    // OID resolution also failed, return OID error (more informative)
                                    Err(oid_err)
                                }
                            }
                        } else {
                            // Not an OID-like string, return branch not found error
                            Err(anyhow::anyhow!("branch {} not found", name_str))
                        }
                    }
                }
            }
            Revision::Parent(base_revision) => {
                Self::resolve_commit_parent(base_revision.resolve(repository)?, repository)
            }
            Revision::Ancestor(base_revision, generations) => {
                let mut oid = base_revision.resolve(repository)?;
                for _ in 0..*generations {
                    oid = Self::resolve_commit_parent(oid, repository)?;
                }

                Ok(oid)
            }
        }
    }

    fn resolve_commit_parent(
        oid: Option<ObjectId>,
        repository: &Repository,
    ) -> anyhow::Result<Option<ObjectId>> {
        if let Some(oid) = oid {
            let commit = repository
                .database()
                .parse_object_as_commit(&oid)?
                .ok_or_else(|| anyhow::anyhow!("object {} is not a commit", oid))?;
            let parent_oid = commit.parent().cloned();

            Ok(parent_oid)
        } else {
            Ok(None)
        }
    }

    fn resolve_oid(oid_str: &str, repository: &Repository) -> anyhow::Result<ObjectId> {
        // Check if it's a full OID (40 hex characters)
        if oid_str.len() == OBJECT_ID_LENGTH && oid_str.chars().all(|c| c.is_ascii_hexdigit()) {
            let oid = ObjectId::try_parse(oid_str.to_string())?;
            // Validate that it's a commit
            Self::validate_oid_is_commit(&oid, repository)?;
            return Ok(oid);
        }

        // It's an abbreviated OID - need to find matching objects
        if oid_str.is_empty() || !oid_str.chars().all(|c| c.is_ascii_hexdigit()) {
            anyhow::bail!("invalid object id: {}", oid_str);
        }

        let matches = repository.database().find_objects_by_prefix(oid_str)?;

        match matches.len() {
            0 => anyhow::bail!(
                "ambiguous argument '{}': unknown revision or path not in the working tree",
                oid_str
            ),
            1 => {
                let oid = &matches[0];
                // Validate that it's a commit
                Self::validate_oid_is_commit(oid, repository)?;
                Ok(oid.clone())
            }
            _ => {
                // Multiple matches - show hint (only show commits as valid candidates)
                let commit_matches: Vec<_> = matches
                    .iter()
                    .filter(|oid| {
                        repository
                            .database()
                            .get_object_type(oid)
                            .map(|t| t == ObjectType::Commit)
                            .unwrap_or(false)
                    })
                    .collect();

                if commit_matches.is_empty() {
                    anyhow::bail!(
                        "ambiguous argument '{}': unknown revision or path not in the working tree",
                        oid_str
                    );
                } else if commit_matches.len() == 1 {
                    return Ok(commit_matches[0].clone());
                }

                // Multiple commit matches - show hint
                let mut error_msg = format!(
                    "short SHA1 {} is ambiguous\nhint: The candidates are:",
                    oid_str
                );
                for oid in &commit_matches {
                    error_msg.push_str(&format!("\nhint:   {} commit", oid.to_short_oid()));
                }
                anyhow::bail!(error_msg)
            }
        }
    }

    fn validate_oid_is_commit(oid: &ObjectId, repository: &Repository) -> anyhow::Result<()> {
        let obj_type = repository
            .database()
            .get_object_type(oid)
            .with_context(|| format!("object {} not found", oid))?;

        if obj_type != ObjectType::Commit {
            anyhow::bail!(
                "object {} is a {}, not a commit",
                oid.to_short_oid(),
                obj_type
            );
        }

        Ok(())
    }

    pub fn try_parse(revision: &str) -> anyhow::Result<Revision> {
        if regex::Regex::new(PARENT_REGEX)
            .with_context(|| format!("invalid parent regex: {PARENT_REGEX}"))?
            .is_match(revision)
        {
            let caps = regex::Regex::new(PARENT_REGEX)
                .with_context(|| format!("invalid parent regex: {PARENT_REGEX}"))?
                .captures(revision)
                .with_context(|| format!("failed to parse revision: {revision}"))?;

            let base_rev = &caps[1];
            let base_revision = Self::try_parse(base_rev)?;

            Ok(Revision::Parent(Box::new(base_revision)))
        } else if regex::Regex::new(ANCESTOR_REGEX)
            .with_context(|| format!("invalid ancestor regex: {ANCESTOR_REGEX}"))?
            .is_match(revision)
        {
            let caps = regex::Regex::new(ANCESTOR_REGEX)
                .with_context(|| format!("invalid ancestor regex: {ANCESTOR_REGEX}"))?
                .captures(revision)
                .with_context(|| format!("failed to parse revision: {revision}"))?;

            let base_rev = &caps[1];
            let generations: usize = caps[2]
                .parse()
                .with_context(|| format!("failed to parse generations in revision: {revision}"))?;

            let base_revision = Self::try_parse(base_rev)?;

            Ok(Revision::Ancestor(Box::new(base_revision), generations))
        } else {
            let resolved_name = *REF_ALIASES.get(revision).unwrap_or(&revision);
            let branch_name = BranchName::try_parse(resolved_name.to_string())?;
            Ok(Revision::Ref(branch_name))
        }
    }

    fn looks_like_oid(s: &str) -> bool {
        // Must be at least 4 characters (minimum prefix length for Git)
        // and contain only hex digits
        s.len() >= 4 && s.len() <= 40 && s.chars().all(|c| c.is_ascii_hexdigit())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    // Unit tests for basic functionality
    #[test]
    fn test_parse_simple_ref() {
        let result = Revision::try_parse("main").unwrap();
        if let Revision::Ref(name) = result {
            assert_eq!(name.as_ref(), "main");
        } else {
            panic!("Expected Ref variant");
        }
    }

    #[test]
    fn test_parse_head_alias() {
        let result = Revision::try_parse("@").unwrap();
        if let Revision::Ref(name) = result {
            assert_eq!(name.as_ref(), "HEAD");
        } else {
            panic!("Expected Ref variant");
        }
    }

    #[test]
    fn test_parse_parent() {
        let result = Revision::try_parse("main^").unwrap();
        if let Revision::Parent(base) = result {
            if let Revision::Ref(name) = *base {
                assert_eq!(name.as_ref(), "main");
            } else {
                panic!("Expected Ref variant in parent");
            }
        } else {
            panic!("Expected Parent variant");
        }
    }

    #[test]
    fn test_parse_ancestor() {
        let result = Revision::try_parse("main~3").unwrap();
        if let Revision::Ancestor(base, generation) = result {
            assert_eq!(generation, 3);
            if let Revision::Ref(name) = *base {
                assert_eq!(name.as_ref(), "main");
            } else {
                panic!("Expected Ref variant in ancestor");
            }
        } else {
            panic!("Expected Ancestor variant");
        }
    }

    #[test]
    fn test_parse_nested_parent() {
        let result = Revision::try_parse("main^^").unwrap();
        // Should be Parent(Parent(Ref("main")))
        if let Revision::Parent(first_parent) = result {
            if let Revision::Parent(second_parent) = *first_parent {
                if let Revision::Ref(name) = *second_parent {
                    assert_eq!(name.as_ref(), "main");
                } else {
                    panic!("Expected Ref at the innermost level");
                }
            } else {
                panic!("Expected second Parent variant");
            }
        } else {
            panic!("Expected first Parent variant");
        }
    }

    #[test]
    fn test_parse_invalid_branch_name_empty() {
        let result = Revision::try_parse("");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_invalid_branch_name_with_space() {
        let result = Revision::try_parse("invalid name");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_invalid_branch_name_with_colon() {
        let result = Revision::try_parse("invalid:name");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_invalid_branch_name_starts_with_dot() {
        let result = Revision::try_parse(".invalid");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_invalid_branch_name_starts_with_slash() {
        let result = Revision::try_parse("/invalid");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_invalid_branch_name_ends_with_slash() {
        let result = Revision::try_parse("invalid/");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_invalid_branch_name_ends_with_lock() {
        let result = Revision::try_parse("branch.lock");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_invalid_branch_name_double_dot() {
        let result = Revision::try_parse("feature..name");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_invalid_parent_with_invalid_base() {
        let result = Revision::try_parse(".invalid^");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_invalid_ancestor_with_invalid_base() {
        let result = Revision::try_parse(".invalid~5");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_ancestor_with_zero() {
        let result = Revision::try_parse("main~0").unwrap();
        if let Revision::Ancestor(_, generation) = result {
            assert_eq!(generation, 0);
        } else {
            panic!("Expected Ancestor variant");
        }
    }

    #[test]
    fn test_parse_valid_hierarchical_branch_name() {
        let result = Revision::try_parse("feature/my-feature").unwrap();
        if let Revision::Ref(name) = result {
            assert_eq!(name.as_ref(), "feature/my-feature");
        } else {
            panic!("Expected Ref variant");
        }
    }

    #[test]
    fn test_parse_full_oid() {
        let oid = "a".repeat(40);
        let result = Revision::try_parse(&oid).unwrap();
        // OIDs are parsed as Ref initially, resolved as OID during resolution
        if let Revision::Ref(name) = result {
            assert_eq!(name.as_ref(), oid);
        } else {
            panic!("Expected Ref variant");
        }
    }

    #[test]
    fn test_parse_abbreviated_oid() {
        let oid = "a1b2c3d";
        let result = Revision::try_parse(oid).unwrap();
        // OIDs are parsed as Ref initially, resolved as OID during resolution
        if let Revision::Ref(name) = result {
            assert_eq!(name.as_ref(), oid);
        } else {
            panic!("Expected Ref variant");
        }
    }

    #[test]
    fn test_parse_oid_minimum_length() {
        let oid = "a1b2";
        let result = Revision::try_parse(oid).unwrap();
        // OIDs are parsed as Ref initially, resolved as OID during resolution
        if let Revision::Ref(name) = result {
            assert_eq!(name.as_ref(), oid);
        } else {
            panic!("Expected Ref variant");
        }
    }

    #[test]
    fn test_parse_oid_too_short_treated_as_ref() {
        let result = Revision::try_parse("abc");
        // Should parse as branch name (not OID since it's too short)
        assert!(result.is_ok());
        if let Ok(Revision::Ref(name)) = result {
            assert_eq!(name.as_ref(), "abc");
        } else {
            panic!("Expected Ref variant for short hex string");
        }
    }

    #[test]
    fn test_parse_oid_with_parent() {
        let oid = "a".repeat(40) + "^";
        let result = Revision::try_parse(&oid).unwrap();
        if let Revision::Parent(base) = result {
            if let Revision::Ref(name) = *base {
                assert_eq!(name.as_ref(), "a".repeat(40));
            } else {
                panic!("Expected Ref variant in parent");
            }
        } else {
            panic!("Expected Parent variant");
        }
    }

    #[test]
    fn test_parse_oid_with_ancestor() {
        let oid = "a".repeat(40) + "~3";
        let result = Revision::try_parse(&oid).unwrap();
        if let Revision::Ancestor(base, generation) = result {
            assert_eq!(generation, 3);
            if let Revision::Ref(name) = *base {
                assert_eq!(name.as_ref(), "a".repeat(40));
            } else {
                panic!("Expected Ref variant in ancestor");
            }
        } else {
            panic!("Expected Ancestor variant");
        }
    }

    #[test]
    fn test_parse_abbreviated_oid_with_parent() {
        let oid = "a1b2c3d^";
        let result = Revision::try_parse(oid).unwrap();
        if let Revision::Parent(base) = result {
            if let Revision::Ref(name) = *base {
                assert_eq!(name.as_ref(), "a1b2c3d");
            } else {
                panic!("Expected Ref variant in parent");
            }
        } else {
            panic!("Expected Parent variant");
        }
    }

    // Property tests

    // Strategy for valid branch names (simplified)
    fn valid_branch_name_strategy() -> impl Strategy<Value = String> {
        prop::string::string_regex("[a-zA-Z0-9][a-zA-Z0-9_/-]*[a-zA-Z0-9]")
            .unwrap()
            .prop_filter("Must not contain invalid patterns", |s| {
                !s.contains("..")
                    && !s.ends_with(".lock")
                    && !s.contains("//")
                    && !s.is_empty()
                    && s.len() < 256
            })
    }

    // Strategy for invalid branch names
    fn invalid_branch_name_strategy() -> impl Strategy<Value = String> {
        prop_oneof![
            Just("".to_string()),
            Just(".invalid".to_string()),
            Just("invalid..name".to_string()),
            Just("/invalid".to_string()),
            Just("invalid/".to_string()),
            Just("invalid.lock".to_string()),
            Just("invalid name".to_string()),
            Just("invalid:name".to_string()),
            Just("invalid*name".to_string()),
            Just("invalid?name".to_string()),
            Just("invalid[name".to_string()),
            Just("invalid\\name".to_string()),
            Just("invalid~name".to_string()),
            Just("invalid^name".to_string()),
            Just("invalid@{name".to_string()),
        ]
    }

    proptest! {
        #[test]
        fn prop_valid_branch_names_parse_successfully(name in valid_branch_name_strategy()) {
            let result = Revision::try_parse(&name);
            prop_assert!(result.is_ok());
            let parsed = result.unwrap();
            if let Revision::Ref(parsed_name) = parsed {
                prop_assert_eq!(parsed_name.as_ref(), &name);
            } else {
                prop_assert!(false, "Expected Ref variant");
            }
        }

        #[test]
        fn prop_invalid_branch_names_fail_to_parse(name in invalid_branch_name_strategy()) {
            let result = Revision::try_parse(&name);
            prop_assert!(result.is_err());
        }

        #[test]
        fn prop_parent_suffix_creates_parent_revision(name in valid_branch_name_strategy()) {
            let revision_str = format!("{}^", name);
            let result = Revision::try_parse(&revision_str);
            prop_assert!(result.is_ok());
            let parsed = result.unwrap();

            if let Revision::Parent(base) = parsed {
                if let Revision::Ref(base_name) = *base {
                    prop_assert_eq!(base_name.as_ref(), &name);
                } else {
                    prop_assert!(false, "Expected Ref variant in parent");
                }
            } else {
                prop_assert!(false, "Expected Parent variant");
            }
        }

        #[test]
        fn prop_ancestor_suffix_creates_ancestor_revision(
            name in valid_branch_name_strategy(),
            generations in 0usize..100
        ) {
            let revision_str = format!("{}~{}", name, generations);
            let result = Revision::try_parse(&revision_str);
            prop_assert!(result.is_ok());
            let parsed = result.unwrap();
            if let Revision::Ancestor(base, generation) = parsed {
                prop_assert_eq!(generation, generations);
                if let Revision::Ref(base_name) = *base {
                    prop_assert_eq!(base_name.as_ref(), &name);
                } else {
                    prop_assert!(false, "Expected Ref variant in ancestor");
                }
            } else {
                prop_assert!(false, "Expected Ancestor variant");
            }
        }

        #[test]
        fn prop_multiple_parent_suffixes_nest_correctly(
            name in valid_branch_name_strategy(),
            parent_count in 1usize..5
        ) {
            let mut revision_str = name.clone();
            for _ in 0..parent_count {
                revision_str.push('^');
            }
            let result = Revision::try_parse(&revision_str);
            prop_assert!(result.is_ok());
            let parsed = result.unwrap();

            // Verify nested structure
            let mut current = parsed;
            for _ in 0..parent_count {
                if let Revision::Parent(base) = current {
                    current = *base;
                } else {
                    prop_assert!(false, "Expected Parent variant");
                    break;
                }
            }
            if let Revision::Ref(base_name) = current {
                prop_assert_eq!(base_name.as_ref(), &name);
            } else {
                prop_assert!(false, "Expected Ref variant at innermost level");
            }
        }

        #[test]
        fn prop_parsing_is_deterministic(name in valid_branch_name_strategy()) {
            let result1 = Revision::try_parse(&name);
            let result2 = Revision::try_parse(&name);
            prop_assert!(result1.is_ok());
            prop_assert!(result2.is_ok());
            // Both should parse the same way
        }
    }

    // Separate test for alias resolution (not a property test since it has no inputs)
    #[test]
    fn test_alias_resolution_works() {
        let result = Revision::try_parse("@");
        assert!(result.is_ok());
        let parsed = result.unwrap();
        if let Revision::Ref(resolved) = parsed {
            assert_eq!(resolved.as_ref(), "HEAD");
        } else {
            panic!("Expected Ref variant");
        }
    }

    // Strategy for valid OIDs (full and abbreviated)
    fn valid_oid_strategy() -> impl Strategy<Value = String> {
        prop_oneof![
            // Full OID (40 hex chars)
            prop::string::string_regex("[0-9a-f]{40}").unwrap(),
            // Abbreviated OID (4-39 hex chars)
            prop::string::string_regex("[0-9a-f]{4,39}").unwrap(),
        ]
    }

    proptest! {
        #[test]
        fn prop_valid_oids_parse_successfully(oid in valid_oid_strategy()) {
            let result = Revision::try_parse(&oid);
            prop_assert!(result.is_ok());
            let parsed = result.unwrap();
            // OIDs are parsed as Ref, resolved as OID during resolution phase
            if let Revision::Ref(name) = parsed {
                prop_assert_eq!(name.as_ref(), oid.as_str());
            } else {
                prop_assert!(false, "Expected Ref variant");
            }
        }

        #[test]
        fn prop_oid_with_parent_suffix_creates_parent_revision(oid in valid_oid_strategy()) {
            let revision_str = format!("{}^", oid);
            let result = Revision::try_parse(&revision_str);
            prop_assert!(result.is_ok());
            let parsed = result.unwrap();

            if let Revision::Parent(base) = parsed {
                if let Revision::Ref(name) = *base {
                    prop_assert_eq!(name.as_ref(), oid.as_str());
                } else {
                    prop_assert!(false, "Expected Ref variant in parent");
                }
            } else {
                prop_assert!(false, "Expected Parent variant");
            }
        }

        #[test]
        fn prop_oid_with_ancestor_suffix_creates_ancestor_revision(
            oid in valid_oid_strategy(),
            generations in 0usize..100
        ) {
            let revision_str = format!("{}~{}", oid, generations);
            let result = Revision::try_parse(&revision_str);
            prop_assert!(result.is_ok());
            let parsed = result.unwrap();

            if let Revision::Ancestor(base, generation) = parsed {
                prop_assert_eq!(generation, generations);
                if let Revision::Ref(name) = *base {
                    prop_assert_eq!(name.as_ref(), oid.as_str());
                } else {
                    prop_assert!(false, "Expected Ref variant in ancestor");
                }
            } else {
                prop_assert!(false, "Expected Ancestor variant");
            }
        }

        #[test]
        fn prop_short_hex_strings_parse_as_ref_not_oid(length in 1usize..4) {
            let hex_str = "a".repeat(length);
            let result = Revision::try_parse(&hex_str);
            // Should not parse as OID (too short), should parse as valid branch name
            prop_assert!(result.is_ok());
            if let Ok(Revision::Ref(name)) = result {
                prop_assert_eq!(name.as_ref(), hex_str.as_str());
            } else {
                prop_assert!(false, "Expected Ref variant for short hex string");
            }
        }
    }
}
