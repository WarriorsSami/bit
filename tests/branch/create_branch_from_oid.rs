use crate::common::command::{
    get_ancestor_commit_id, get_parent_commit_id, repository_with_multiple_commits, run_bit_command,
};
use assert_fs::TempDir;
use pretty_assertions::assert_eq;
use rstest::rstest;

#[rstest]
fn create_branch_from_full_oid(
    repository_with_multiple_commits: TempDir,
) -> Result<(), Box<dyn std::error::Error>> {
    let repository_dir = repository_with_multiple_commits;

    // Get HEAD commit ID
    let head_path = repository_dir.path().join(".git").join("HEAD");
    let head_content = std::fs::read_to_string(&head_path)?.trim().to_string();

    // Get the parent of HEAD from the database
    let parent_id = get_parent_commit_id(repository_dir.path(), &head_content)?;

    // Create a branch from the full OID of HEAD's parent
    run_bit_command(
        repository_dir.path(),
        &["branch", "feature-from-full-oid", &parent_id],
    )
    .assert()
    .success();

    // Verify the branch was created and points to the correct commit
    let branch_path = repository_dir
        .path()
        .join(".git")
        .join("refs")
        .join("heads")
        .join("feature-from-full-oid");
    assert!(branch_path.exists());
    let branch_content = std::fs::read_to_string(&branch_path)?.trim().to_string();

    // Verify it points to the parent commit
    assert_eq!(branch_content, parent_id);

    // Verify it's different from HEAD
    assert_ne!(branch_content, head_content);

    Ok(())
}

#[rstest]
fn create_branch_from_abbreviated_oid(
    repository_with_multiple_commits: TempDir,
) -> Result<(), Box<dyn std::error::Error>> {
    let repository_dir = repository_with_multiple_commits;

    // Get HEAD commit ID
    let head_path = repository_dir.path().join(".git").join("HEAD");
    let head_content = std::fs::read_to_string(&head_path)?.trim().to_string();

    // Get the parent of HEAD from the database
    let parent_id = get_parent_commit_id(repository_dir.path(), &head_content)?;

    // Use an abbreviated OID (first 7 characters)
    let abbreviated_oid = &parent_id[..7];

    // Create a branch from the abbreviated OID
    run_bit_command(
        repository_dir.path(),
        &["branch", "feature-from-abbrev-oid", abbreviated_oid],
    )
    .assert()
    .success();

    // Verify the branch was created and points to the correct commit
    let branch_path = repository_dir
        .path()
        .join(".git")
        .join("refs")
        .join("heads")
        .join("feature-from-abbrev-oid");
    assert!(branch_path.exists());
    let branch_content = std::fs::read_to_string(&branch_path)?.trim().to_string();

    // Verify it points to the full parent commit ID
    assert_eq!(branch_content, parent_id);

    // Verify it's different from HEAD
    assert_ne!(branch_content, head_content);

    Ok(())
}

#[rstest]
fn create_branch_from_oid_with_parent_suffix(
    repository_with_multiple_commits: TempDir,
) -> Result<(), Box<dyn std::error::Error>> {
    let repository_dir = repository_with_multiple_commits;

    // Get HEAD commit ID
    let head_path = repository_dir.path().join(".git").join("HEAD");
    let head_content = std::fs::read_to_string(&head_path)?.trim().to_string();

    // Get the parent of HEAD from the database
    let parent_id = get_parent_commit_id(repository_dir.path(), &head_content)?;

    // Get the grandparent (parent of parent)
    let grandparent_id = get_parent_commit_id(repository_dir.path(), &parent_id)?;

    // Create a branch using OID^ syntax
    let oid_with_parent = format!("{}^", parent_id);
    run_bit_command(
        repository_dir.path(),
        &["branch", "feature-from-oid-parent", &oid_with_parent],
    )
    .assert()
    .success();

    // Verify the branch was created and points to the grandparent
    let branch_path = repository_dir
        .path()
        .join(".git")
        .join("refs")
        .join("heads")
        .join("feature-from-oid-parent");
    assert!(branch_path.exists());
    let branch_content = std::fs::read_to_string(&branch_path)?.trim().to_string();

    // Verify it points to the grandparent commit
    assert_eq!(branch_content, grandparent_id);

    Ok(())
}

#[rstest]
fn create_branch_from_oid_with_ancestor_suffix(
    repository_with_multiple_commits: TempDir,
) -> Result<(), Box<dyn std::error::Error>> {
    let repository_dir = repository_with_multiple_commits;

    // Get HEAD commit ID
    let head_path = repository_dir.path().join(".git").join("HEAD");
    let head_content = std::fs::read_to_string(&head_path)?.trim().to_string();

    // Get HEAD~3 from the database
    let ancestor_id = get_ancestor_commit_id(repository_dir.path(), &head_content, 3)?;

    // Create a branch using abbreviated OID~2 syntax
    let abbreviated_head = &head_content[..10];
    let oid_with_ancestor = format!("{}~3", abbreviated_head);
    run_bit_command(
        repository_dir.path(),
        &["branch", "feature-from-oid-ancestor", &oid_with_ancestor],
    )
    .assert()
    .success();

    // Verify the branch was created and points to HEAD~3
    let branch_path = repository_dir
        .path()
        .join(".git")
        .join("refs")
        .join("heads")
        .join("feature-from-oid-ancestor");
    assert!(branch_path.exists());
    let branch_content = std::fs::read_to_string(&branch_path)?.trim().to_string();

    // Verify it points to the ancestor commit
    assert_eq!(branch_content, ancestor_id);

    Ok(())
}

#[rstest]
fn create_branch_from_minimum_length_abbreviated_oid(
    repository_with_multiple_commits: TempDir,
) -> Result<(), Box<dyn std::error::Error>> {
    let repository_dir = repository_with_multiple_commits;

    // Get HEAD commit ID
    let head_path = repository_dir.path().join(".git").join("HEAD");
    let head_content = std::fs::read_to_string(&head_path)?.trim().to_string();

    // Get the parent of HEAD from the database
    let parent_id = get_parent_commit_id(repository_dir.path(), &head_content)?;

    // Use minimum length abbreviated OID (4 characters)
    let min_abbreviated_oid = &parent_id[..4];

    // Create a branch from the minimum abbreviated OID
    run_bit_command(
        repository_dir.path(),
        &["branch", "feature-from-min-oid", min_abbreviated_oid],
    )
    .assert()
    .success();

    // Verify the branch was created and points to the correct commit
    let branch_path = repository_dir
        .path()
        .join(".git")
        .join("refs")
        .join("heads")
        .join("feature-from-min-oid");
    assert!(branch_path.exists());
    let branch_content = std::fs::read_to_string(&branch_path)?.trim().to_string();

    // Verify it points to the full parent commit ID
    assert_eq!(branch_content, parent_id);

    Ok(())
}

#[rstest]
fn create_branch_from_nonexistent_oid_fails(
    repository_with_multiple_commits: TempDir,
) -> Result<(), Box<dyn std::error::Error>> {
    let repository_dir = repository_with_multiple_commits;

    // Use a non-existent OID
    let nonexistent_oid = "deadbeef";

    // Attempt to create a branch from the non-existent OID
    let output = run_bit_command(
        repository_dir.path(),
        &["branch", "feature-from-nonexistent", nonexistent_oid],
    )
    .assert()
    .failure();

    // Verify the error message
    let stderr = String::from_utf8(output.get_output().stderr.clone())?;
    assert!(stderr.contains("unknown revision"));

    Ok(())
}

#[rstest]
fn create_branch_from_invalid_oid_fails(
    repository_with_multiple_commits: TempDir,
) -> Result<(), Box<dyn std::error::Error>> {
    let repository_dir = repository_with_multiple_commits;

    // Use an invalid OID (contains non-hex characters)
    let invalid_oid = "xyz123";

    // Attempt to create a branch from the invalid OID
    // This should fail because it's not valid hex and too short to be a valid branch name
    run_bit_command(
        repository_dir.path(),
        &["branch", "feature-from-invalid", invalid_oid],
    )
    .assert()
    .failure();

    Ok(())
}
