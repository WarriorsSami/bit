use crate::common::command::{init_repository_dir, run_bit_command};
use assert_fs::TempDir;
use pretty_assertions::assert_eq;
use rstest::rstest;

#[rstest]
fn create_branch_with_valid_name(
    init_repository_dir: TempDir,
) -> Result<(), Box<dyn std::error::Error>> {
    let repository_dir = init_repository_dir;

    // assert the refs path exists and it's an empty directory
    let refs_path = repository_dir.path().join(".git").join("refs");
    assert!(refs_path.exists());
    assert!(refs_path.is_dir());
    let entries = std::fs::read_dir(&refs_path)?;
    assert_eq!(entries.count(), 0);

    // assert the HEAD file exists and it's a non-empty file
    let head_path = repository_dir.path().join(".git").join("HEAD");
    assert!(head_path.exists());
    assert!(head_path.is_file());
    let head_content = std::fs::read_to_string(&head_path)?;
    assert!(!head_content.is_empty());

    // create the master branch
    let branch_name = "master";
    run_bit_command(repository_dir.path(), &["branch", branch_name])
        .assert()
        .success();

    // assert the branch ref exists and its content is equal to the HEAD's content
    let branch_ref_path = refs_path.join("heads").join(branch_name);
    assert!(branch_ref_path.exists());
    let branch_ref_content = std::fs::read_to_string(&branch_ref_path)?;
    assert_eq!(branch_ref_content, head_content);

    Ok(())
}
