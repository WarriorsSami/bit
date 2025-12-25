use crate::common::command::{get_head_commit_sha, init_repository_dir, run_bit_command};
use assert_fs::TempDir;
use pretty_assertions::assert_eq;
use rstest::rstest;

#[rstest]
fn create_branch_with_hierarchical_name(
    init_repository_dir: TempDir,
) -> Result<(), Box<dyn std::error::Error>> {
    let repository_dir = init_repository_dir;

    // Get HEAD commit ID (resolve symbolic references)
    let head_content = get_head_commit_sha(repository_dir.path())?;

    // create a branch with hierarchical name (containing slashes)
    let branch_name = "feature/new-feature";
    run_bit_command(repository_dir.path(), &["branch", branch_name])
        .assert()
        .success();

    // assert the branch ref exists with proper directory structure
    let branch_ref_path = repository_dir
        .path()
        .join(".git")
        .join("refs")
        .join("heads")
        .join("feature")
        .join("new-feature");
    assert!(branch_ref_path.exists());
    let branch_ref_content = std::fs::read_to_string(&branch_ref_path)?;
    assert_eq!(branch_ref_content, head_content);

    Ok(())
}

#[rstest]
fn create_branch_with_deeply_nested_hierarchical_name(
    init_repository_dir: TempDir,
) -> Result<(), Box<dyn std::error::Error>> {
    let repository_dir = init_repository_dir;

    // Get HEAD commit ID (resolve symbolic references)
    let head_content = get_head_commit_sha(repository_dir.path())?;

    // create a branch with deeply nested hierarchical name
    let branch_name = "team/backend/feature/user-authentication";
    run_bit_command(repository_dir.path(), &["branch", branch_name])
        .assert()
        .success();

    // assert the branch ref exists with proper directory structure
    let branch_ref_path = repository_dir
        .path()
        .join(".git")
        .join("refs")
        .join("heads")
        .join("team")
        .join("backend")
        .join("feature")
        .join("user-authentication");
    assert!(branch_ref_path.exists());
    let branch_ref_content = std::fs::read_to_string(&branch_ref_path)?;
    assert_eq!(branch_ref_content, head_content);

    Ok(())
}
