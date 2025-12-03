use crate::artifacts::branch::INVALID_BRANCH_NAME_REGEX;
use anyhow::Context;

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
