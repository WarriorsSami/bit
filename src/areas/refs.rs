use crate::artifacts::branch::branch_name::{BranchName, SymRefName};
use crate::artifacts::objects::object_id::ObjectId;
use anyhow::Context;
use derive_new::new;
use file_guard::Lock;
use std::collections::HashMap;
use std::io::Write;
use std::ops::DerefMut;
use std::path::Path;
use walkdir::WalkDir;

#[derive(Debug, new)]
pub struct Refs {
    path: Box<Path>,
}

const SYMREF_REGEX: &str = r"^ref: (.+)$";
pub const HEAD_REF_NAME: &str = "HEAD";

#[derive(Debug, Clone)]
enum SymRefOrOid {
    SymRef { sym_ref_name: SymRefName },
    Oid(ObjectId),
}

impl SymRefOrOid {
    fn read_symref_or_oid(path: &Path) -> anyhow::Result<Option<SymRefOrOid>> {
        if !path.exists() {
            return Ok(None);
        }

        let content = std::fs::read_to_string(path)?;
        let content = content.trim();

        if content.is_empty() {
            return Ok(None);
        }

        let symref_match = regex::Regex::new(SYMREF_REGEX)?.captures(content);
        if let Some(symref_match) = symref_match {
            Ok(Some(SymRefOrOid::SymRef {
                sym_ref_name: SymRefName::new(symref_match[1].to_string()),
            }))
        } else {
            Ok(Some(SymRefOrOid::Oid(ObjectId::try_parse(
                content.to_string(),
            )?)))
        }
    }
}

impl Refs {
    pub fn is_current_branch(&self, branch_name: &BranchName) -> anyhow::Result<bool> {
        let current_ref = self.current_ref(None)?;

        Ok(branch_name == &BranchName::try_parse_sym_ref_name(&current_ref)?)
    }

    pub fn read_oid(&self, sym_ref_name: &SymRefName) -> anyhow::Result<Option<ObjectId>> {
        self.read_ref(BranchName::try_parse_sym_ref_name(sym_ref_name)?)
    }

    pub fn current_ref(&self, source: Option<SymRefName>) -> anyhow::Result<SymRefName> {
        let source = source.unwrap_or_else(|| SymRefName::new("HEAD".to_string()));

        let ref_content =
            SymRefOrOid::read_symref_or_oid(self.path.join(source.as_ref_path()).as_path())?;

        match ref_content {
            Some(SymRefOrOid::SymRef { sym_ref_name }) => Ok(self.current_ref(Some(sym_ref_name))?),
            Some(_) | None => Ok(source),
        }
    }

    fn read_symref(&self, path: &Path) -> anyhow::Result<Option<ObjectId>> {
        let ref_content = SymRefOrOid::read_symref_or_oid(path)?;

        match ref_content {
            Some(SymRefOrOid::SymRef { sym_ref_name }) => {
                self.read_symref(self.path.join(sym_ref_name.as_ref_path()).as_path())
            }
            Some(SymRefOrOid::Oid(oid)) => Ok(Some(oid)),
            None => Ok(None),
        }
    }

    fn update_symref(&self, path: &Path, oid: ObjectId) -> anyhow::Result<()> {
        let mut ref_file = std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .open(path)
            .with_context(|| format!("failed to open ref file at {:?}", path))?;
        let mut lock = file_guard::lock(&mut ref_file, Lock::Exclusive, 0, 1)?;

        let ref_content = SymRefOrOid::read_symref_or_oid(path)?;

        match ref_content {
            Some(SymRefOrOid::SymRef { sym_ref_name }) => {
                let target_path = self.path.join(sym_ref_name.as_ref_path());
                self.update_symref(target_path.as_path(), oid)
            }
            Some(SymRefOrOid::Oid(_)) | None => {
                lock.deref_mut().write_all(oid.as_ref().as_bytes())?;
                Ok(())
            }
        }
    }

    pub fn set_head(&self, revision: &str, raw_ref: String) -> anyhow::Result<()> {
        let revision_path = self.heads_path().join(revision).into_boxed_path();

        if revision_path.exists() {
            self.update_ref_file(self.head_path(), format!("ref: refs/heads/{}", revision))
        } else {
            self.update_ref_file(self.head_path(), raw_ref)
        }
    }

    pub fn update_head(&self, oid: ObjectId) -> anyhow::Result<()> {
        self.update_symref(self.head_path().as_ref(), oid)
    }

    pub fn read_head(&self) -> anyhow::Result<Option<ObjectId>> {
        self.read_symref(&self.head_path())
    }

    pub fn update_ref_file(&self, path: Box<Path>, raw_ref: String) -> anyhow::Result<()> {
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
        lock.deref_mut().write_all(raw_ref.as_bytes())?;

        Ok(())
    }

    pub fn read_ref(&self, branch_name: BranchName) -> anyhow::Result<Option<ObjectId>> {
        let ref_path = self.find_path_to_branch(branch_name)?;
        self.read_symref(&ref_path)
    }

    fn find_path_to_branch(&self, branch_name: BranchName) -> anyhow::Result<Box<Path>> {
        // search for the branch ref file in .git, .git/refs and .git/refs/heads
        [self.path.clone(), self.refs_path(), self.heads_path()]
            .iter()
            .map(|base_path| base_path.join(branch_name.as_ref()).into_boxed_path())
            .find(|path| path.exists())
            .ok_or_else(|| anyhow::anyhow!("branch {} not found", branch_name))
    }

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

    pub fn create_branch(&self, name: BranchName, source_oid: ObjectId) -> anyhow::Result<()> {
        let branch_path = self.heads_path().join(name.as_ref()).into_boxed_path();

        // check whether another branch with the same name already exists
        if branch_path.exists() && !name.is_default_branch() {
            anyhow::bail!("branch {} already exists", name);
        }

        self.update_ref_file(branch_path, source_oid.as_ref().into())
    }

    pub fn delete_branch(&self, name: &BranchName) -> anyhow::Result<ObjectId> {
        let branch_path = self.heads_path().join(name.as_ref()).into_boxed_path();

        let oid = self.read_symref(branch_path.as_ref())?;
        match oid {
            Some(oid) => {
                std::fs::remove_file(branch_path.as_ref()).with_context(|| {
                    format!("failed to delete branch file at {:?}", branch_path)
                })?;
                self.prune_branch_empty_parent_dirs(branch_path.as_ref())?;

                Ok(oid)
            }
            None => anyhow::bail!("branch {} does not exist", name),
        }
    }

    pub fn list_branches(&self) -> anyhow::Result<Vec<SymRefName>> {
        self.list_refs(self.heads_path().as_ref())
    }

    fn list_refs(&self, path: &Path) -> anyhow::Result<Vec<SymRefName>> {
        Ok(WalkDir::new(path)
            .into_iter()
            .filter_map(|entry| entry.ok())
            .filter_map(|entry| {
                if entry.path().is_file() {
                    let relative_path = entry.path().strip_prefix(self.path.as_ref()).ok()?;
                    Some(SymRefName::new(relative_path.to_string_lossy().to_string()))
                } else {
                    None
                }
            })
            .collect::<Vec<_>>())
    }

    pub fn reverse_refs(&self) -> anyhow::Result<HashMap<ObjectId, Vec<SymRefName>>> {
        Ok(self
            .list_all_refs()?
            .into_iter()
            .fold(HashMap::new(), |mut acc, sym_ref| {
                if let Ok(Some(oid)) = self.read_oid(&sym_ref) {
                    acc.entry(oid).or_insert_with(Vec::new).push(sym_ref);
                }
                acc
            }))
    }

    fn list_all_refs(&self) -> anyhow::Result<Vec<SymRefName>> {
        Ok(self
            .list_refs(self.refs_path().as_ref())?
            .into_iter()
            .chain(std::iter::once(SymRefName::new(HEAD_REF_NAME.to_string())))
            .collect::<Vec<_>>())
    }

    fn prune_branch_empty_parent_dirs(&self, path: &Path) -> anyhow::Result<()> {
        if let Some(parent) = path.parent()
            && parent != self.heads_path().as_ref()
            && parent.read_dir()?.next().is_none()
        {
            std::fs::remove_dir(parent).with_context(|| {
                format!("failed to remove empty branch directory at {:?}", parent)
            })?;
            self.prune_branch_empty_parent_dirs(parent)?;
        }

        Ok(())
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
    use crate::artifacts::branch::branch_name::BranchName;
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
