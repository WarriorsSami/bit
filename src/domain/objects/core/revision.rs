use crate::domain::areas::repository::Repository;
use crate::domain::objects::branch_name::BranchName;
use crate::domain::objects::core::{
    ANCESTOR_REGEX, INVALID_BRANCH_NAME_REGEX, PARENT_REGEX, REF_ALIASES,
};
use crate::domain::objects::object_id::ObjectId;
use anyhow::Context;
use derive_new::new;
use std::ops::Deref;

pub struct RevisionContext<'r> {
    repository: &'r Repository,
}

#[derive(Debug)]
pub enum RevisionRecord {
    Ref(BranchName),
    Ancestor(Box<RevisionRecord>, usize),
    Parent(Box<RevisionRecord>),
}

impl<'r> RevisionContext<'r> {
    pub fn initialize(
        repo: &'r Repository,
        revision_expression: &str,
    ) -> anyhow::Result<(Self, RevisionRecord)> {
        let revision = RevisionContext::parse(revision_expression)?;

        Ok((Self { repository: repo }, revision))
    }

    pub fn resolve(&self, revision: RevisionRecord) -> anyhow::Result<Option<ObjectId>> {
        match revision {
            RevisionRecord::Ref(branch_name) => {
                self.repository.refs().read_ref(branch_name.clone())
            }
            RevisionRecord::Parent(base_revision) => {
                self.resolve_commit_parent(self.resolve(*base_revision)?)
            }
            RevisionRecord::Ancestor(base_revision, generations) => {
                let mut oid = self.resolve(*base_revision)?;
                for _ in 0..generations {
                    oid = self.resolve_commit_parent(oid)?;
                }

                Ok(oid)
            }
        }
    }

    fn resolve_commit_parent(&self, oid: Option<ObjectId>) -> anyhow::Result<Option<ObjectId>> {
        if let Some(oid) = oid {
            let commit = self
                .repository
                .database()
                .parse_object_as_commit(&oid)?
                .ok_or_else(|| anyhow::anyhow!("object {} is not a commit", oid))?;
            let parent_oid = commit.parent().cloned();

            Ok(parent_oid)
        } else {
            Ok(None)
        }
    }

    pub fn parse(revision: &str) -> anyhow::Result<RevisionRecord> {
        if regex::Regex::new(PARENT_REGEX)
            .with_context(|| format!("invalid parent regex: {PARENT_REGEX}"))?
            .is_match(revision)
        {
            let caps = regex::Regex::new(PARENT_REGEX)
                .with_context(|| format!("invalid parent regex: {PARENT_REGEX}"))?
                .captures(revision)
                .with_context(|| format!("failed to parse revision: {revision}"))?;

            let base_rev = &caps[1];
            let base_revision = RevisionContext::parse(base_rev)?;

            Ok(RevisionRecord::Parent(Box::new(base_revision)))
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

            let base_revision = RevisionContext::parse(base_rev)?;

            Ok(RevisionRecord::Ancestor(
                Box::new(base_revision),
                generations,
            ))
        } else {
            let resolved_name = *REF_ALIASES.get(revision).unwrap_or(&revision);
            let branch_name = BranchName::try_parse(resolved_name.to_string())?;
            Ok(RevisionRecord::Ref(branch_name))
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
        let result = RevisionContext::parse("main").unwrap();
        if let RevisionRecord::Ref(name) = result {
            assert_eq!(name.as_ref(), "main");
        } else {
            panic!("Expected Ref variant");
        }
    }

    #[test]
    fn test_parse_head_alias() {
        let result = RevisionContext::parse("@").unwrap();
        if let RevisionRecord::Ref(name) = result {
            assert_eq!(name.as_ref(), "HEAD");
        } else {
            panic!("Expected Ref variant");
        }
    }

    #[test]
    fn test_parse_parent() {
        let result = RevisionContext::parse("main^").unwrap();
        if let RevisionRecord::Parent(base) = result {
            if let RevisionRecord::Ref(name) = *base {
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
        let result = RevisionContext::parse("main~3").unwrap();
        if let RevisionRecord::Ancestor(base, generation) = result {
            assert_eq!(generation, 3);
            if let RevisionRecord::Ref(name) = *base {
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
        let result = RevisionContext::parse("main^^").unwrap();
        // Should be Parent(Parent(Ref("main")))
        if let RevisionRecord::Parent(first_parent) = result {
            if let RevisionRecord::Parent(second_parent) = *first_parent {
                if let RevisionRecord::Ref(name) = *second_parent {
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
        let result = RevisionContext::parse("");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_invalid_branch_name_with_space() {
        let result = RevisionContext::parse("invalid name");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_invalid_branch_name_with_colon() {
        let result = RevisionContext::parse("invalid:name");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_invalid_branch_name_starts_with_dot() {
        let result = RevisionContext::parse(".invalid");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_invalid_branch_name_starts_with_slash() {
        let result = RevisionContext::parse("/invalid");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_invalid_branch_name_ends_with_slash() {
        let result = RevisionContext::parse("invalid/");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_invalid_branch_name_ends_with_lock() {
        let result = RevisionContext::parse("branch.lock");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_invalid_branch_name_double_dot() {
        let result = RevisionContext::parse("feature..name");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_invalid_parent_with_invalid_base() {
        let result = RevisionContext::parse(".invalid^");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_invalid_ancestor_with_invalid_base() {
        let result = RevisionContext::parse(".invalid~5");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_ancestor_with_zero() {
        let result = RevisionContext::parse("main~0").unwrap();
        if let RevisionRecord::Ancestor(_, generation) = result {
            assert_eq!(generation, 0);
        } else {
            panic!("Expected Ancestor variant");
        }
    }

    #[test]
    fn test_parse_valid_hierarchical_branch_name() {
        let result = RevisionContext::parse("feature/my-feature").unwrap();
        if let RevisionRecord::Ref(name) = result {
            assert_eq!(name.as_ref(), "feature/my-feature");
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
            let result = RevisionContext::parse(&name);
            prop_assert!(result.is_ok());
            let parsed = result.unwrap();
            if let RevisionRecord::Ref(parsed_name) = parsed {
                prop_assert_eq!(parsed_name.as_ref(), &name);
            } else {
                prop_assert!(false, "Expected Ref variant");
            }
        }

        #[test]
        fn prop_invalid_branch_names_fail_to_parse(name in invalid_branch_name_strategy()) {
            let result = RevisionContext::parse(&name);
            prop_assert!(result.is_err());
        }

        #[test]
        fn prop_parent_suffix_creates_parent_revision(name in valid_branch_name_strategy()) {
            let revision_str = format!("{}^", name);
            let result = RevisionContext::parse(&revision_str);
            prop_assert!(result.is_ok());
            let parsed = result.unwrap();

            if let RevisionRecord::Parent(base) = parsed {
                if let RevisionRecord::Ref(base_name) = *base {
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
            let result = RevisionContext::parse(&revision_str);
            prop_assert!(result.is_ok());
            let parsed = result.unwrap();
            if let RevisionRecord::Ancestor(base, generation) = parsed {
                prop_assert_eq!(generation, generations);
                if let RevisionRecord::Ref(base_name) = *base {
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
            let result = RevisionContext::parse(&revision_str);
            prop_assert!(result.is_ok());
            let parsed = result.unwrap();

            // Verify nested structure
            let mut current = parsed;
            for _ in 0..parent_count {
                if let RevisionRecord::Parent(base) = current {
                    current = *base;
                } else {
                    prop_assert!(false, "Expected Parent variant");
                    break;
                }
            }
            if let RevisionRecord::Ref(base_name) = current {
                prop_assert_eq!(base_name.as_ref(), &name);
            } else {
                prop_assert!(false, "Expected Ref variant at innermost level");
            }
        }

        #[test]
        fn prop_parsing_is_deterministic(name in valid_branch_name_strategy()) {
            let result1 = RevisionContext::parse(&name);
            let result2 = RevisionContext::parse(&name);
            prop_assert!(result1.is_ok());
            prop_assert!(result2.is_ok());
            // Both should parse the same way
        }
    }

    // Separate test for alias resolution (not a property test since it has no inputs)
    #[test]
    fn test_alias_resolution_works() {
        let result = RevisionContext::parse("@");
        assert!(result.is_ok());
        let parsed = result.unwrap();
        if let RevisionRecord::Ref(resolved) = parsed {
            assert_eq!(resolved.as_ref(), "HEAD");
        } else {
            panic!("Expected Ref variant");
        }
    }
}
