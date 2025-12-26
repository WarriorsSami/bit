use crate::common::command::{
    get_ancestor_commit_id, get_head_commit_sha, get_parent_commit_id,
    repository_with_multiple_commits, run_bit_command,
};
use assert_fs::TempDir;
use pretty_assertions::assert_eq;
use rstest::rstest;

#[rstest]
fn create_branch_from_ancestor_with_generation_1(
    repository_with_multiple_commits: TempDir,
) -> Result<(), Box<dyn std::error::Error>> {
    let repository_dir = repository_with_multiple_commits;

    // Get HEAD commit ID (resolve symbolic references)
    let head_content = get_head_commit_sha(repository_dir.path())?;

    // Get HEAD~1 and HEAD^ from the database (should be the same)
    let expected_ancestor_id = get_ancestor_commit_id(repository_dir.path(), &head_content, 1)?;
    let expected_parent_id = get_parent_commit_id(repository_dir.path(), &head_content)?;

    // They should be equal
    assert_eq!(expected_ancestor_id, expected_parent_id);

    // Create a branch from HEAD~1 (same as HEAD^)
    run_bit_command(
        repository_dir.path(),
        &["branch", "create", "ancestor-1", "HEAD~1"],
    )
    .assert()
    .success();

    // Create a branch from HEAD^ for comparison
    run_bit_command(
        repository_dir.path(),
        &["branch", "create", "parent", "HEAD^"],
    )
    .assert()
    .success();

    // Verify both branches point to the same commit
    let ancestor_path = repository_dir
        .path()
        .join(".git")
        .join("refs")
        .join("heads")
        .join("ancestor-1");
    let parent_path = repository_dir
        .path()
        .join(".git")
        .join("refs")
        .join("heads")
        .join("parent");

    let ancestor_content = std::fs::read_to_string(&ancestor_path)?.trim().to_string();
    let parent_content = std::fs::read_to_string(&parent_path)?.trim().to_string();
    assert_eq!(ancestor_content, parent_content);

    // Verify against actual database values
    assert_eq!(ancestor_content, expected_ancestor_id);
    assert_eq!(parent_content, expected_parent_id);

    // Verify they're different from HEAD
    assert_ne!(ancestor_content, head_content);

    Ok(())
}

#[rstest]
fn create_branch_from_ancestor_with_generation_2(
    repository_with_multiple_commits: TempDir,
) -> Result<(), Box<dyn std::error::Error>> {
    let repository_dir = repository_with_multiple_commits;

    // Get HEAD commit ID (resolve symbolic references)
    let head_content = get_head_commit_sha(repository_dir.path())?;

    // Get HEAD~2 from the database (should be same as HEAD^^)
    let expected_ancestor_id = get_ancestor_commit_id(repository_dir.path(), &head_content, 2)?;

    // Create a branch from HEAD~2 (grandparent)
    run_bit_command(
        repository_dir.path(),
        &["branch", "create", "ancestor-2", "HEAD~2"],
    )
    .assert()
    .success();

    // Create a branch from HEAD^^ for comparison
    run_bit_command(
        repository_dir.path(),
        &["branch", "create", "grandparent", "HEAD^^"],
    )
    .assert()
    .success();

    // Verify both branches point to the same commit
    let ancestor_path = repository_dir
        .path()
        .join(".git")
        .join("refs")
        .join("heads")
        .join("ancestor-2");
    let grandparent_path = repository_dir
        .path()
        .join(".git")
        .join("refs")
        .join("heads")
        .join("grandparent");

    let ancestor_content = std::fs::read_to_string(&ancestor_path)?.trim().to_string();
    let grandparent_content = std::fs::read_to_string(&grandparent_path)?
        .trim()
        .to_string();
    assert_eq!(ancestor_content, grandparent_content);

    // Verify against actual database value
    assert_eq!(ancestor_content, expected_ancestor_id);
    assert_eq!(grandparent_content, expected_ancestor_id);

    // Verify they're different from HEAD
    assert_ne!(ancestor_content, head_content);

    Ok(())
}

#[rstest]
fn create_branch_from_ancestor_with_generation_3(
    repository_with_multiple_commits: TempDir,
) -> Result<(), Box<dyn std::error::Error>> {
    let repository_dir = repository_with_multiple_commits;

    // Get HEAD commit ID (resolve symbolic references)
    let head_content = get_head_commit_sha(repository_dir.path())?;

    // Get HEAD~3 from the database
    let expected_ancestor_id = get_ancestor_commit_id(repository_dir.path(), &head_content, 3)?;

    // Create a branch from HEAD~3 (great-grandparent)
    run_bit_command(
        repository_dir.path(),
        &["branch", "create", "ancestor-3", "HEAD~3"],
    )
    .assert()
    .success();

    // Verify the branch was created
    let ancestor_path = repository_dir
        .path()
        .join(".git")
        .join("refs")
        .join("heads")
        .join("ancestor-3");
    assert!(ancestor_path.exists());

    // Verify it points to the actual ancestor from database
    let ancestor_content = std::fs::read_to_string(&ancestor_path)?.trim().to_string();
    assert_eq!(ancestor_content, expected_ancestor_id);

    // Verify it's different from HEAD
    assert_ne!(ancestor_content, head_content);

    Ok(())
}

#[rstest]
fn create_branch_from_ancestor_of_branch(
    repository_with_multiple_commits: TempDir,
) -> Result<(), Box<dyn std::error::Error>> {
    let repository_dir = repository_with_multiple_commits;

    // Get HEAD commit ID (resolve symbolic references)
    let head_content = get_head_commit_sha(repository_dir.path())?;

    // Create a branch from HEAD
    run_bit_command(repository_dir.path(), &["branch", "create", "main"])
        .assert()
        .success();

    let main_path = repository_dir
        .path()
        .join(".git")
        .join("refs")
        .join("heads")
        .join("main");
    let main_content = std::fs::read_to_string(&main_path)?.trim().to_string();

    // main should point to HEAD
    assert_eq!(main_content, head_content);

    // Get main~2 from the database
    let expected_ancestor_id = get_ancestor_commit_id(repository_dir.path(), &main_content, 2)?;

    // Create a branch from main~2
    run_bit_command(
        repository_dir.path(),
        &["branch", "create", "ancestor-of-main", "main~2"],
    )
    .assert()
    .success();

    // Verify the branch was created
    let ancestor_path = repository_dir
        .path()
        .join(".git")
        .join("refs")
        .join("heads")
        .join("ancestor-of-main");
    assert!(ancestor_path.exists());

    // Verify it points to the actual ancestor from database
    let ancestor_content = std::fs::read_to_string(&ancestor_path)?.trim().to_string();
    assert_eq!(ancestor_content, expected_ancestor_id);

    // Verify it's different from main
    assert_ne!(ancestor_content, main_content);

    Ok(())
}

#[rstest]
fn create_branch_from_ancestor_with_generation_0(
    repository_with_multiple_commits: TempDir,
) -> Result<(), Box<dyn std::error::Error>> {
    let repository_dir = repository_with_multiple_commits;

    // Create a branch from HEAD
    run_bit_command(repository_dir.path(), &["branch", "create", "main"])
        .assert()
        .success();

    // Create a branch from HEAD~0 (should be same as HEAD)
    run_bit_command(
        repository_dir.path(),
        &["branch", "create", "ancestor-0", "HEAD~0"],
    )
    .assert()
    .success();

    // Verify both branches point to the same commit
    let ancestor_path = repository_dir
        .path()
        .join(".git")
        .join("refs")
        .join("heads")
        .join("ancestor-0");
    let main_path = repository_dir
        .path()
        .join(".git")
        .join("refs")
        .join("heads")
        .join("main");

    let ancestor_content = std::fs::read_to_string(&ancestor_path)?;
    let main_content = std::fs::read_to_string(&main_path)?;
    assert_eq!(ancestor_content, main_content);

    Ok(())
}

#[rstest]
fn create_branch_from_ancestor_with_alias(
    repository_with_multiple_commits: TempDir,
) -> Result<(), Box<dyn std::error::Error>> {
    let repository_dir = repository_with_multiple_commits;

    // Create a branch from ancestor using @ alias (@~2)
    run_bit_command(
        repository_dir.path(),
        &["branch", "create", "from-alias-ancestor", "@~2"],
    )
    .assert()
    .success();

    // Verify the branch was created
    let branch_path = repository_dir
        .path()
        .join(".git")
        .join("refs")
        .join("heads")
        .join("from-alias-ancestor");
    assert!(branch_path.exists());

    // Verify it points to the actual ancestor from database
    let ancestor_content = std::fs::read_to_string(&branch_path)?;
    let expected_ancestor_id = get_ancestor_commit_id(repository_dir.path(), "HEAD", 2)?;

    assert_eq!(ancestor_content, expected_ancestor_id);

    Ok(())
}

#[rstest]
fn create_branch_from_ancestor_beyond_history_fails(
    repository_with_multiple_commits: TempDir,
) -> Result<(), Box<dyn std::error::Error>> {
    let repository_dir = repository_with_multiple_commits;

    // Try to go beyond the history (we have 4 commits, so ~4 should fail)
    run_bit_command(
        repository_dir.path(),
        &["branch", "create", "too-far", "HEAD~4"],
    )
    .assert()
    .failure();

    Ok(())
}

#[rstest]
fn create_branch_from_ancestor_of_nonexistent_ref_fails(
    repository_with_multiple_commits: TempDir,
) -> Result<(), Box<dyn std::error::Error>> {
    let repository_dir = repository_with_multiple_commits;

    // Try to create a branch from ancestor of non-existent branch
    run_bit_command(
        repository_dir.path(),
        &["branch", "create", "new-branch", "nonexistent~2"],
    )
    .assert()
    .failure();

    Ok(())
}
