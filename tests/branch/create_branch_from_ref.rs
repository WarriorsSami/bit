use crate::common::command::{
    get_ancestor_commit_id, get_head_commit_sha, get_parent_commit_id,
    repository_with_multiple_commits, run_bit_command,
};
use assert_fs::TempDir;
use pretty_assertions::assert_eq;
use rstest::rstest;

#[rstest]
fn create_branch_from_head_ref(
    repository_with_multiple_commits: TempDir,
) -> Result<(), Box<dyn std::error::Error>> {
    let repository_dir = repository_with_multiple_commits;

    // Get HEAD commit ID (resolve symbolic references)
    let head_content = get_head_commit_sha(repository_dir.path())?;

    // Get the parent of HEAD from the database
    let expected_parent_id = get_parent_commit_id(repository_dir.path(), &head_content)?;

    // Create a branch from HEAD^ (parent of current HEAD)
    run_bit_command(
        repository_dir.path(),
        &["branch", "feature-from-head", "HEAD^"],
    )
    .assert()
    .success();

    // Verify the branch was created and points to HEAD^ (not HEAD)
    let branch_path = repository_dir
        .path()
        .join(".git")
        .join("refs")
        .join("heads")
        .join("feature-from-head");
    assert!(branch_path.exists());
    let branch_content = std::fs::read_to_string(&branch_path)?.trim().to_string();

    // Verify against actual parent from database
    assert_eq!(branch_content, expected_parent_id);

    // Verify it's different from HEAD
    assert_ne!(branch_content, head_content);

    Ok(())
}

#[rstest]
fn create_branch_from_head_alias(
    repository_with_multiple_commits: TempDir,
) -> Result<(), Box<dyn std::error::Error>> {
    let repository_dir = repository_with_multiple_commits;

    // Get HEAD commit ID (resolve symbolic references)
    let head_content = get_head_commit_sha(repository_dir.path())?;

    // Get HEAD~2 from the database
    let expected_ancestor_id = get_ancestor_commit_id(repository_dir.path(), &head_content, 2)?;

    // Create a new branch using @ alias for HEAD~2 (@~2)
    run_bit_command(
        repository_dir.path(),
        &["branch", "feature-from-alias", "@~2"],
    )
    .assert()
    .success();

    // Verify the branch was created and points to HEAD~2 (not HEAD)
    let branch_path = repository_dir
        .path()
        .join(".git")
        .join("refs")
        .join("heads")
        .join("feature-from-alias");
    assert!(branch_path.exists());
    let branch_content = std::fs::read_to_string(&branch_path)?.trim().to_string();

    // Verify against actual ancestor from database
    assert_eq!(branch_content, expected_ancestor_id);

    // Verify it's different from HEAD
    assert_ne!(branch_content, head_content);

    Ok(())
}

#[rstest]
fn create_branch_from_another_branch(
    repository_with_multiple_commits: TempDir,
) -> Result<(), Box<dyn std::error::Error>> {
    let repository_dir = repository_with_multiple_commits;

    // Get HEAD commit ID (resolve symbolic references)
    let head_content = get_head_commit_sha(repository_dir.path())?;

    // Get the parent of HEAD from the database
    let expected_parent_id = get_parent_commit_id(repository_dir.path(), &head_content)?;

    // Create the first branch from HEAD^ (not HEAD)
    run_bit_command(repository_dir.path(), &["branch", "main", "HEAD^"])
        .assert()
        .success();

    let main_branch_path = repository_dir
        .path()
        .join(".git")
        .join("refs")
        .join("heads")
        .join("main");
    let main_branch_content = std::fs::read_to_string(&main_branch_path)?
        .trim()
        .to_string();

    // Verify main points to expected parent
    assert_eq!(main_branch_content, expected_parent_id);

    // Create a second branch from the first branch
    run_bit_command(repository_dir.path(), &["branch", "feature", "main"])
        .assert()
        .success();

    // Verify the second branch points to the same commit as the first
    let feature_branch_path = repository_dir
        .path()
        .join(".git")
        .join("refs")
        .join("heads")
        .join("feature");
    assert!(feature_branch_path.exists());
    let feature_branch_content = std::fs::read_to_string(&feature_branch_path)?
        .trim()
        .to_string();
    assert_eq!(feature_branch_content, main_branch_content);
    assert_eq!(feature_branch_content, expected_parent_id);

    // Verify both are different from HEAD
    assert_ne!(feature_branch_content, head_content);
    assert_ne!(main_branch_content, head_content);

    Ok(())
}

#[rstest]
fn create_branch_from_nonexistent_ref_fails(
    repository_with_multiple_commits: TempDir,
) -> Result<(), Box<dyn std::error::Error>> {
    let repository_dir = repository_with_multiple_commits;

    // Try to create a branch from a non-existent branch
    run_bit_command(
        repository_dir.path(),
        &["branch", "new-branch", "nonexistent"],
    )
    .assert()
    .failure();

    Ok(())
}
