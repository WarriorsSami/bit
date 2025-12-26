use crate::common::command::{
    get_ancestor_commit_id, get_head_commit_sha, get_parent_commit_id,
    repository_with_multiple_commits, run_bit_command,
};
use assert_fs::TempDir;
use pretty_assertions::{assert_eq, assert_ne};
use rstest::rstest;

#[rstest]
fn create_branch_from_parent_of_head(
    repository_with_multiple_commits: TempDir,
) -> Result<(), Box<dyn std::error::Error>> {
    let repository_dir = repository_with_multiple_commits;

    // Get HEAD commit ID (resolve symbolic references)
    let head_content = get_head_commit_sha(repository_dir.path())?;

    // Get the parent of HEAD from the database
    let expected_parent_id = get_parent_commit_id(repository_dir.path(), &head_content)?;

    // Create a branch from the parent of HEAD (HEAD^)
    run_bit_command(
        repository_dir.path(),
        &["branch", "create", "parent-branch", "HEAD^"],
    )
    .assert()
    .success();

    // Verify the branch was created
    let branch_path = repository_dir
        .path()
        .join(".git")
        .join("refs")
        .join("heads")
        .join("parent-branch");
    assert!(branch_path.exists());

    // The parent branch should point to the actual parent from database
    let branch_content = std::fs::read_to_string(&branch_path)?.trim().to_string();
    assert_eq!(branch_content, expected_parent_id);

    // Verify it's different from HEAD
    assert_ne!(branch_content, head_content);

    Ok(())
}

#[rstest]
fn create_branch_from_parent_of_branch(
    repository_with_multiple_commits: TempDir,
) -> Result<(), Box<dyn std::error::Error>> {
    let repository_dir = repository_with_multiple_commits;

    // Get HEAD commit ID (resolve symbolic references)
    let head_content = get_head_commit_sha(repository_dir.path())?;

    // Get HEAD~2 from the database
    let expected_main_id = get_ancestor_commit_id(repository_dir.path(), &head_content, 2)?;

    // Get the parent of HEAD~2 from the database
    let expected_parent_of_main_id =
        get_parent_commit_id(repository_dir.path(), &expected_main_id)?;

    // Create a branch from HEAD~2
    run_bit_command(
        repository_dir.path(),
        &["branch", "create", "main", "HEAD~2"],
    )
    .assert()
    .success();

    let main_branch_path = repository_dir
        .path()
        .join(".git")
        .join("refs")
        .join("heads")
        .join("main");
    let main_content = std::fs::read_to_string(&main_branch_path)?
        .trim()
        .to_string();

    // Verify main points to expected ancestor
    assert_eq!(main_content, expected_main_id);

    // Create a branch from the parent of main (main^)
    run_bit_command(
        repository_dir.path(),
        &["branch", "create", "parent-of-main", "main^"],
    )
    .assert()
    .success();

    // Verify the branch was created
    let parent_branch_path = repository_dir
        .path()
        .join(".git")
        .join("refs")
        .join("heads")
        .join("parent-of-main");
    assert!(parent_branch_path.exists());

    // The parent branch should point to the actual parent of main from database
    let parent_content = std::fs::read_to_string(&parent_branch_path)?
        .trim()
        .to_string();
    assert_eq!(parent_content, expected_parent_of_main_id);
    assert_ne!(parent_content, main_content);

    // Verify both are different from HEAD
    assert_ne!(parent_content, head_content);
    assert_ne!(main_content, head_content);

    Ok(())
}

#[rstest]
fn create_branch_from_grandparent_of_head(
    repository_with_multiple_commits: TempDir,
) -> Result<(), Box<dyn std::error::Error>> {
    let repository_dir = repository_with_multiple_commits;

    // Get HEAD commit ID (resolve symbolic references)
    let head_content = get_head_commit_sha(repository_dir.path())?;

    // Get the grandparent of HEAD from the database (HEAD^^)
    let expected_grandparent_id = get_ancestor_commit_id(repository_dir.path(), &head_content, 2)?;

    // Create a branch from the grandparent of HEAD (HEAD^^)
    run_bit_command(
        repository_dir.path(),
        &["branch", "create", "grandparent", "HEAD^^"],
    )
    .assert()
    .success();

    // Verify the branch was created
    let grandparent_path = repository_dir
        .path()
        .join(".git")
        .join("refs")
        .join("heads")
        .join("grandparent");
    assert!(grandparent_path.exists());

    // Verify it points to the actual grandparent from database
    let grandparent_content = std::fs::read_to_string(&grandparent_path)?
        .trim()
        .to_string();
    assert_eq!(grandparent_content, expected_grandparent_id);

    // Verify it's different from HEAD
    assert_ne!(grandparent_content, head_content);

    Ok(())
}

#[rstest]
fn create_branch_from_parent_with_alias(
    repository_with_multiple_commits: TempDir,
) -> Result<(), Box<dyn std::error::Error>> {
    let repository_dir = repository_with_multiple_commits;

    // Create a branch from the parent of HEAD using @ alias (@^)
    run_bit_command(
        repository_dir.path(),
        &["branch", "create", "from-alias-parent", "@^"],
    )
    .assert()
    .success();

    // Verify the branch was created
    let branch_path = repository_dir
        .path()
        .join(".git")
        .join("refs")
        .join("heads")
        .join("from-alias-parent");
    assert!(branch_path.exists());

    Ok(())
}

#[rstest]
fn create_branch_from_parent_of_nonexistent_ref_fails(
    repository_with_multiple_commits: TempDir,
) -> Result<(), Box<dyn std::error::Error>> {
    let repository_dir = repository_with_multiple_commits;

    // Try to create a branch from parent of non-existent branch
    run_bit_command(
        repository_dir.path(),
        &["branch", "create", "new-branch", "nonexistent^"],
    )
    .assert()
    .failure();

    Ok(())
}
