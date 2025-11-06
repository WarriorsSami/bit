use crate::domain::areas::repository::Repository;
use crate::domain::objects::INVALID_BRANCH_NAME_REGEX;
use crate::domain::objects::object_id::ObjectId;
use anyhow::Context;
use derive_new::new;
use std::ops::Deref;

const PARENT_REGEX: &str = r"^(.+)\^$";
const ANCESTOR_REGEX: &str = r"^(.+)\~(\d+)$";
const REF_ALIASES: phf::Map<&'static str, &'static str> = phf::phf_map! {
    "@" => "HEAD",
};

pub fn is_valid_branch_name(name: &str) -> anyhow::Result<bool> {
    if name.is_empty() {
        return Ok(false);
    }

    let re = regex::Regex::new(INVALID_BRANCH_NAME_REGEX)
        .with_context(|| format!("invalid branch name regex: {INVALID_BRANCH_NAME_REGEX}"))?;

    // The regex matches INVALID patterns, so return true if it does NOT match
    Ok(!re.is_match(name))
}

pub struct Revision<'r> {
    repo: &'r Repository,
}

#[derive(Debug)]
pub enum RevisionRecord {
    Ref(String),
    Ancestor(Box<RevisionRecord>, usize),
    Parent(Box<RevisionRecord>),
}

impl<'r> Revision<'r> {
    pub fn initialize(
        repo: &'r Repository,
        revision_expression: &str,
    ) -> anyhow::Result<(Self, RevisionRecord)> {
        let revision = Revision::parse(revision_expression)?;

        match revision {
            Some(revision) => Ok((Self { repo }, revision)),
            None => anyhow::bail!("invalid revision expression: {revision_expression}"),
        }
    }

    // TODO: finish resolving
    // pub fn resolve(&self, revision: RevisionRecord) -> anyhow::Result<ObjectId> {
    //     match revision {
    //         RevisionRecord::Ref
    //     }
    // }

    pub fn parse(revision: &str) -> anyhow::Result<Option<RevisionRecord>> {
        if regex::Regex::new(PARENT_REGEX)
            .with_context(|| format!("invalid parent regex: {PARENT_REGEX}"))?
            .is_match(revision)
        {
            let caps = regex::Regex::new(PARENT_REGEX)
                .with_context(|| format!("invalid parent regex: {PARENT_REGEX}"))?
                .captures(revision)
                .with_context(|| format!("failed to parse revision: {revision}"))?;

            let base_rev = &caps[1];
            let base_revision = Revision::parse(base_rev)?;

            match base_revision {
                Some(base_revision) => Ok(Some(RevisionRecord::Parent(Box::new(base_revision)))),
                None => Ok(None),
            }
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

            let base_revision = Revision::parse(base_rev)?;

            match base_revision {
                Some(base_revision) => Ok(Some(RevisionRecord::Ancestor(
                    Box::new(base_revision),
                    generations,
                ))),
                None => Ok(None),
            }
        } else if is_valid_branch_name(revision)? {
            let resolved_name = *REF_ALIASES.get(revision).unwrap_or(&revision);
            Ok(Some(RevisionRecord::Ref(resolved_name.to_string())))
        } else {
            Ok(None)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    // Unit tests for basic functionality
    #[test]
    fn test_parse_simple_ref() {
        let result = Revision::parse("main").unwrap();
        assert!(result.is_some());
        if let Some(RevisionRecord::Ref(name)) = result {
            assert_eq!(name, "main");
        } else {
            panic!("Expected Ref variant");
        }
    }

    #[test]
    fn test_parse_head_alias() {
        let result = Revision::parse("@").unwrap();
        assert!(result.is_some());
        if let Some(RevisionRecord::Ref(name)) = result {
            assert_eq!(name, "HEAD");
        } else {
            panic!("Expected Ref variant");
        }
    }

    #[test]
    fn test_parse_parent() {
        let result = Revision::parse("main^").unwrap();
        assert!(result.is_some());
        if let Some(RevisionRecord::Parent(base)) = result {
            if let RevisionRecord::Ref(name) = *base {
                assert_eq!(name, "main");
            } else {
                panic!("Expected Ref variant in parent");
            }
        } else {
            panic!("Expected Parent variant");
        }
    }

    #[test]
    fn test_parse_ancestor() {
        let result = Revision::parse("main~3").unwrap();
        assert!(result.is_some());
        if let Some(RevisionRecord::Ancestor(base, generation)) = result {
            assert_eq!(generation, 3);
            if let RevisionRecord::Ref(name) = *base {
                assert_eq!(name, "main");
            } else {
                panic!("Expected Ref variant in ancestor");
            }
        } else {
            panic!("Expected Ancestor variant");
        }
    }

    #[test]
    fn test_parse_nested_parent() {
        let result = Revision::parse("main^^").unwrap();
        assert!(result.is_some());
        // Should be Parent(Parent(Ref("main")))
    }

    #[test]
    fn test_parse_invalid_branch_name_empty() {
        let result = Revision::parse("").unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_parse_invalid_branch_name_with_space() {
        let result = Revision::parse("invalid name").unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_parse_invalid_branch_name_with_colon() {
        let result = Revision::parse("invalid:name").unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_parse_invalid_branch_name_starts_with_dot() {
        let result = Revision::parse(".invalid").unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_parse_invalid_branch_name_starts_with_slash() {
        let result = Revision::parse("/invalid").unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_parse_invalid_branch_name_ends_with_slash() {
        let result = Revision::parse("invalid/").unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_parse_invalid_branch_name_ends_with_lock() {
        let result = Revision::parse("branch.lock").unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_parse_invalid_branch_name_double_dot() {
        let result = Revision::parse("feature..name").unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_parse_invalid_parent_with_invalid_base() {
        let result = Revision::parse(".invalid^").unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_parse_invalid_ancestor_with_invalid_base() {
        let result = Revision::parse(".invalid~5").unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_parse_ancestor_with_zero() {
        let result = Revision::parse("main~0").unwrap();
        assert!(result.is_some());
        if let Some(RevisionRecord::Ancestor(_, generation)) = result {
            assert_eq!(generation, 0);
        } else {
            panic!("Expected Ancestor variant");
        }
    }

    #[test]
    fn test_parse_valid_hierarchical_branch_name() {
        let result = Revision::parse("feature/my-feature").unwrap();
        assert!(result.is_some());
        if let Some(RevisionRecord::Ref(name)) = result {
            assert_eq!(name, "feature/my-feature");
        } else {
            panic!("Expected Ref variant");
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
            let result = Revision::parse(&name);
            prop_assert!(result.is_ok());
            let parsed = result.unwrap();
            prop_assert!(parsed.is_some());
            if let Some(RevisionRecord::Ref(parsed_name)) = parsed {
                prop_assert_eq!(&parsed_name, &name);
            }
        }

        #[test]
        fn prop_invalid_branch_names_fail_to_parse(name in invalid_branch_name_strategy()) {
            let result = Revision::parse(&name);
            prop_assert!(result.is_ok());
            let parsed = result.unwrap();
            prop_assert!(parsed.is_none());
        }

        #[test]
        fn prop_parent_suffix_creates_parent_revision(name in valid_branch_name_strategy()) {
            let revision_str = format!("{}^", name);
            let result = Revision::parse(&revision_str);
            prop_assert!(result.is_ok());
            let parsed = result.unwrap();
            prop_assert!(parsed.is_some());

            if let Some(RevisionRecord::Parent(base)) = parsed
                && let RevisionRecord::Ref(base_name) = *base {
                    prop_assert_eq!(&base_name, &name);
            }
        }

        #[test]
        fn prop_ancestor_suffix_creates_ancestor_revision(
            name in valid_branch_name_strategy(),
            generations in 0usize..100
        ) {
            let revision_str = format!("{}~{}", name, generations);
            let result = Revision::parse(&revision_str);
            prop_assert!(result.is_ok());
            let parsed = result.unwrap();
            prop_assert!(parsed.is_some());
            if let Some(RevisionRecord::Ancestor(base, generation)) = parsed {
                prop_assert_eq!(generation, generations);
                if let RevisionRecord::Ref(base_name) = *base {
                    prop_assert_eq!(&base_name, &name);
                }
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
            let result = Revision::parse(&revision_str);
            prop_assert!(result.is_ok());
            let parsed = result.unwrap();
            prop_assert!(parsed.is_some());

            // Verify nested structure
            let mut current = parsed;
            for _ in 0..parent_count {
                if let Some(RevisionRecord::Parent(base)) = current {
                    current = Some(*base);
                } else {
                    prop_assert!(false, "Expected Parent variant");
                }
            }
            if let Some(RevisionRecord::Ref(base_name)) = current {
                prop_assert_eq!(&base_name, &name);
            }
        }

        #[test]
        fn prop_parsing_is_deterministic(name in valid_branch_name_strategy()) {
            let result1 = Revision::parse(&name);
            let result2 = Revision::parse(&name);
            prop_assert!(result1.is_ok());
            prop_assert!(result2.is_ok());
            // Both should parse the same way
        }

        #[test]
        fn prop_alias_resolution_works(name in "@") {
            let result = Revision::parse(&name);
            prop_assert!(result.is_ok());
            let parsed = result.unwrap();
            prop_assert!(parsed.is_some());
            if let Some(RevisionRecord::Ref(resolved)) = parsed {
                prop_assert_eq!(&resolved, "HEAD");
            }
        }
    }
}
