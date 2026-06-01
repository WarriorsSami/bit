use crate::artifacts::branch::INVALID_BRANCH_NAME_REGEX;
use colored::Colorize;
use derive_new::new;

#[derive(Debug, thiserror::Error)]
pub enum BranchNameError {
    #[error("branch name cannot be empty")]
    Empty,
    #[error("invalid branch name: {0}")]
    InvalidName(String),
    #[error("symbolic ref name must start with 'refs/heads/' or 'HEAD', got '{0}'")]
    InvalidSymRef(String),
    #[error("invalid branch name regex: {0}")]
    InvalidRegex(#[from] regex::Error),
}

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

    pub fn to_branch_name(&self) -> Result<BranchName, BranchNameError> {
        BranchName::try_parse_sym_ref_name(self)
    }

    pub fn to_short_name(&self) -> Result<String, BranchNameError> {
        self.to_branch_name().map(|b| b.as_ref().to_string())
    }

    pub fn to_colored_name(&self, name: String) -> String {
        let colored_name = if self.is_detached_head() {
            name.bold().cyan()
        } else {
            name.bold().green()
        };

        format!("{colored_name}")
    }
}

impl AsRef<str> for SymRefName {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, PartialOrd, Ord)]
pub struct BranchName(String);

impl BranchName {
    pub fn try_parse(name: String) -> Result<Self, BranchNameError> {
        if name.is_empty() {
            return Err(BranchNameError::Empty);
        }

        let re = regex::Regex::new(INVALID_BRANCH_NAME_REGEX)?;

        if re.is_match(&name) {
            Err(BranchNameError::InvalidName(name))
        } else {
            Ok(Self(name))
        }
    }

    pub fn try_parse_sym_ref_name(sym_ref_name: &SymRefName) -> Result<Self, BranchNameError> {
        if !sym_ref_name.0.starts_with(REF_PREFIX) && !sym_ref_name.0.starts_with("HEAD") {
            return Err(BranchNameError::InvalidSymRef(sym_ref_name.0.clone()));
        }

        let sym_ref_name = sym_ref_name.0.trim_start_matches(REF_PREFIX);
        Self::try_parse(sym_ref_name.to_string())
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
