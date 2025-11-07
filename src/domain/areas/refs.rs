use crate::domain::objects::branch_name::BranchName;
use crate::domain::objects::object_id::ObjectId;
use anyhow::Context;
use derive_new::new;
use file_guard::Lock;
use std::io::Write;
use std::ops::DerefMut;
use std::path::Path;

#[derive(Debug, new)]
pub struct Refs {
    path: Box<Path>,
}

impl Refs {
    pub fn update_head(&self, oid: ObjectId) -> anyhow::Result<()> {
        self.update_ref_file(self.head_path(), oid)
    }

    pub fn read_head(&self) -> anyhow::Result<Option<ObjectId>> {
        self.read_ref_file(self.head_path())
    }

    pub fn update_ref_file(&self, path: Box<Path>, oid: ObjectId) -> anyhow::Result<()> {
        // create all the parent directories if they don't exist
        std::fs::create_dir_all(path.parent().with_context(|| {
            format!(
                "failed to create parent directories for ref file at {:?}",
                path
            )
        })?)?;

        // open the ref file as WRONLY and CREAT to write commit_id to it
        let mut ref_file = std::fs::OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(path.clone())
            .with_context(|| format!("failed to open ref file at {:?}", path))?;
        let mut lock = file_guard::lock(&mut ref_file, Lock::Exclusive, 0, 1)?;
        lock.deref_mut().write_all(oid.as_ref().as_bytes())?;

        Ok(())
    }

    // TODO: finish reading ref by branch name
    // TODO: add branch name as new type
    pub fn read_ref(&self) {}

    fn read_ref_file(&self, path: Box<Path>) -> anyhow::Result<Option<ObjectId>> {
        // read the ref file content
        let content = std::fs::read_to_string(path.clone())
            .with_context(|| format!("failed to read ref file at {:?}", path))?;
        let content = content.trim();

        if content.starts_with("ref: ") {
            Ok(None)
        } else {
            Ok(Some(ObjectId::try_parse(content.to_string())?))
        }
    }

    pub fn create_branch(&self, name: BranchName) -> anyhow::Result<()> {
        let branch_path = self.heads_path().join(name.as_ref()).into_boxed_path();

        // check whether another branch with the same already exists
        if branch_path.exists() {
            anyhow::bail!("branch {} already exists", name);
        }

        // update the branch ref file with the HEAD content
        let oid = self.read_head()?;
        self.update_ref_file(branch_path, oid.with_context(|| "failed to read HEAD")?)
    }

    pub fn head_path(&self) -> Box<Path> {
        self.path.join("HEAD").into_boxed_path()
    }

    pub fn refs_path(&self) -> Box<Path> {
        self.path.join("refs").into_boxed_path()
    }

    pub fn heads_path(&self) -> Box<Path> {
        self.refs_path().join("heads").into_boxed_path()
    }
}

#[cfg(test)]
mod tests {
    use crate::domain::objects::branch_name::BranchName;
    use proptest::proptest;

    proptest! {
        #[test]
        fn test_is_valid_branch_name_with_valid_branch_name(
            branch_name in "[a-zA-Z0-9_-]+"
        ) {
            // Valid names: alphanumeric, underscore, hyphen
            assert!(BranchName::try_parse(branch_name).is_ok());
        }

        #[test]
        fn test_is_valid_branch_name_with_slashes(
            prefix in "[a-zA-Z0-9_-]+",
            suffix in "[a-zA-Z0-9_-]+"
        ) {
            // Valid names can have slashes: feature/branch-name
            let branch_name = format!("{}/{}", prefix, suffix);
            assert!(BranchName::try_parse(branch_name).is_ok());
        }

        #[test]
        fn test_is_invalid_branch_name_starting_with_dot(
            suffix in "[a-zA-Z0-9_-]+"
        ) {
            // Invalid: starts with dot
            let branch_name = format!(".{}", suffix);
            assert!(BranchName::try_parse(branch_name).is_err());
        }

        #[test]
        fn test_is_invalid_branch_name_ending_with_lock(
            prefix in "[a-zA-Z0-9_-]+"
        ) {
            // Invalid: ends with .lock
            let branch_name = format!("{}.lock", prefix);
            assert!(BranchName::try_parse(branch_name).is_err());
        }

        #[test]
        fn test_is_invalid_branch_name_with_consecutive_dots(
            prefix in "[a-zA-Z0-9_-]+",
            suffix in "[a-zA-Z0-9_-]+"
        ) {
            // Invalid: consecutive dots
            let branch_name = format!("{}..{}", prefix, suffix);
            assert!(BranchName::try_parse(branch_name).is_err());
        }

        #[test]
        fn test_is_invalid_branch_name_with_slash_dot(
            prefix in "[a-zA-Z0-9_-]+",
            suffix in "[a-zA-Z0-9_-]+"
        ) {
            // Invalid: contains /.
            let branch_name = format!("{}/.{}", prefix, suffix);
            assert!(BranchName::try_parse(branch_name).is_err());
        }

        #[test]
        fn test_is_invalid_branch_name_starting_with_slash(
            suffix in "[a-zA-Z0-9_-]+"
        ) {
            // Invalid: starts with /
            let branch_name = format!("/{}", suffix);
            assert!(BranchName::try_parse(branch_name).is_err());
        }

        #[test]
        fn test_is_invalid_branch_name_ending_with_slash(
            prefix in "[a-zA-Z0-9_-]+"
        ) {
            // Invalid: ends with /
            let branch_name = format!("{}/", prefix);
            assert!(BranchName::try_parse(branch_name).is_err());
        }

        #[test]
        fn test_is_invalid_branch_name_with_at_brace(
            prefix in "[a-zA-Z0-9_-]+",
            suffix in "[a-zA-Z0-9_-]+"
        ) {
            // Invalid: contains @{
            let branch_name = format!("{}@{{{}}}", prefix, suffix);
            assert!(BranchName::try_parse(branch_name).is_err());
        }

        #[test]
        fn test_is_invalid_branch_name_with_control_chars(
            prefix in "[a-zA-Z0-9_-]+",
            suffix in "[a-zA-Z0-9_-]+"
        ) {
            // Invalid: contains control characters
            let branch_name = format!("{}\x00{}", prefix, suffix);
            assert!(BranchName::try_parse(branch_name).is_err());
        }

        #[test]
        fn test_is_invalid_branch_name_with_special_chars(
            prefix in "[a-zA-Z0-9_-]+",
            suffix in "[a-zA-Z0-9_-]+",
            special_char in r"[\*:\?\[\\^~]"
        ) {
            // Invalid: contains special characters
            let branch_name = format!("{}{}{}", prefix, special_char, suffix);
            assert!(BranchName::try_parse(branch_name).is_err());
        }
    }

    #[test]
    fn test_is_invalid_branch_name_empty() {
        // Invalid: empty string
        assert!(BranchName::try_parse("".to_string()).is_err());
    }

    #[test]
    fn test_is_valid_branch_name_simple() {
        // Valid: simple names
        assert!(BranchName::try_parse("main".to_string()).is_ok());
        assert!(BranchName::try_parse("feature-123".to_string()).is_ok());
        assert!(BranchName::try_parse("my_branch".to_string()).is_ok());
    }

    #[test]
    fn test_is_valid_branch_name_with_path() {
        // Valid: hierarchical names
        assert!(BranchName::try_parse("feature/new-feature".to_string()).is_ok());
        assert!(BranchName::try_parse("bugfix/issue-123".to_string()).is_ok());
    }
}
