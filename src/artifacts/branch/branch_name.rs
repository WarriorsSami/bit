use crate::artifacts::branch::INVALID_BRANCH_NAME_REGEX;
use anyhow::Context;
use derive_new::new;

const REF_PREFIX: &str = "refs/heads/";

#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, PartialOrd, Ord, new)]
pub struct SymRefName(String);

impl SymRefName {
    pub fn is_detached_head(&self) -> bool {
        self.0.starts_with("HEAD")
    }

    pub fn as_ref_path(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, PartialOrd, Ord)]
pub struct BranchName(String);

impl BranchName {
    pub fn try_parse(name: String) -> anyhow::Result<Self> {
        if name.is_empty() {
            anyhow::bail!("branch name cannot be empty");
        }

        let re = regex::Regex::new(INVALID_BRANCH_NAME_REGEX)
            .with_context(|| format!("invalid branch name regex: {INVALID_BRANCH_NAME_REGEX}"))?;

        if re.is_match(&name) {
            anyhow::bail!("invalid branch name: {}", name);
        } else {
            Ok(Self(name))
        }
    }

    pub fn try_parse_sym_ref_name(sym_ref_name: &SymRefName) -> anyhow::Result<Self> {
        if !sym_ref_name.0.starts_with(REF_PREFIX) && !sym_ref_name.0.starts_with("HEAD") {
            anyhow::bail!(
                "symbolic ref name must start with '{}' or 'HEAD', got '{}'",
                REF_PREFIX,
                sym_ref_name.0
            );
        }

        let sym_ref_name = sym_ref_name.0.trim_start_matches(REF_PREFIX);
        Self::try_parse(sym_ref_name.parse()?)
    }

    pub fn is_default_branch(&self) -> bool {
        self.0 == "master" || self.0 == "main"
    }
}

impl AsRef<str> for BranchName {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for BranchName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}
