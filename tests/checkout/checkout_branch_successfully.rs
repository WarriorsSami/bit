use crate::common::command::{bit_commit, run_bit_command};
use crate::common::file::{FileSpec, write_file};
use assert_fs::TempDir;
use pretty_assertions::assert_eq;
use rstest::{fixture, rstest};

/// Create a repository with multiple commits and branches for checkout testing
#[fixture]
pub fn repository_with_branches() -> TempDir {
    crate::common::redirect_temp_dir();
    let repository_dir = TempDir::new().expect("Failed to create temp dir");

    run_bit_command(repository_dir.path(), &["init"])
        .assert()
        .success();

    // First commit - create initial files
    let file1 = FileSpec::new(
        repository_dir.path().join("file1.txt"),
        "initial content 1".to_string(),
    );
    write_file(file1);

    let file2 = FileSpec::new(
        repository_dir.path().join("file2.txt"),
        "initial content 2".to_string(),
    );
    write_file(file2);

    run_bit_command(repository_dir.path(), &["add", "."])
        .assert()
        .success();

    bit_commit(repository_dir.path(), "Initial commit")
        .assert()
        .success();

    // Create master branch at this point
    run_bit_command(repository_dir.path(), &["branch", "master"])
        .assert()
        .success();

    // Second commit - modify file1 and add file3
    let file1_modified = FileSpec::new(
        repository_dir.path().join("file1.txt"),
        "modified content 1".to_string(),
    );
    write_file(file1_modified);

    let file3 = FileSpec::new(
        repository_dir.path().join("file3.txt"),
        "new content 3".to_string(),
    );
    write_file(file3);

    run_bit_command(repository_dir.path(), &["add", "."])
        .assert()
        .success();

    bit_commit(repository_dir.path(), "Second commit")
        .assert()
        .success();

    // Create feature branch at this point
    run_bit_command(repository_dir.path(), &["branch", "feature"])
        .assert()
        .success();

    // Third commit - modify file2
    let file2_modified = FileSpec::new(
        repository_dir.path().join("file2.txt"),
        "modified content 2".to_string(),
    );
    write_file(file2_modified);

    run_bit_command(repository_dir.path(), &["add", "."])
        .assert()
        .success();

    bit_commit(repository_dir.path(), "Third commit")
        .assert()
        .success();

    repository_dir
}

#[rstest]
fn checkout_branch_successfully(
    repository_with_branches: TempDir,
) -> Result<(), Box<dyn std::error::Error>> {
    let repository_dir = repository_with_branches;

    // Get the current HEAD commit (should be at third commit)
    let head_path = repository_dir.path().join(".git").join("HEAD");
    let initial_head = std::fs::read_to_string(&head_path)?;

    // Get the feature branch commit SHA
    let feature_branch_path = repository_dir
        .path()
        .join(".git")
        .join("refs")
        .join("heads")
        .join("feature");
    let feature_commit = std::fs::read_to_string(&feature_branch_path)?;

    // Verify initial workspace state (at third commit)
    let file1_content = std::fs::read_to_string(repository_dir.path().join("file1.txt"))?;
    assert_eq!(file1_content, "modified content 1");

    let file2_content = std::fs::read_to_string(repository_dir.path().join("file2.txt"))?;
    assert_eq!(file2_content, "modified content 2");

    let file3_path = repository_dir.path().join("file3.txt");
    assert!(file3_path.exists());

    // Checkout the feature branch (which is at the second commit)
    run_bit_command(repository_dir.path(), &["checkout", "feature"])
        .assert()
        .success();

    // Verify HEAD is updated to point to the feature branch commit
    let updated_head = std::fs::read_to_string(&head_path)?;
    assert_eq!(updated_head.trim(), feature_commit.trim());
    assert_ne!(updated_head, initial_head);

    // Verify workspace files are updated to match the feature branch state
    // file1 should still be modified (it was modified in second commit)
    let file1_after_checkout = std::fs::read_to_string(repository_dir.path().join("file1.txt"))?;
    assert_eq!(file1_after_checkout, "modified content 1");

    // file2 should be reverted to initial state (it was modified in third commit)
    let file2_after_checkout = std::fs::read_to_string(repository_dir.path().join("file2.txt"))?;
    assert_eq!(file2_after_checkout, "initial content 2");

    // file3 should still exist (it was added in second commit)
    let file3_path = repository_dir.path().join("file3.txt");
    assert!(file3_path.exists());
    let file3_after_checkout = std::fs::read_to_string(file3_path)?;
    assert_eq!(file3_after_checkout, "new content 3");

    Ok(())
}

#[rstest]
fn checkout_branch_by_commit_sha(
    repository_with_branches: TempDir,
) -> Result<(), Box<dyn std::error::Error>> {
    let repository_dir = repository_with_branches;

    // Get the master branch commit SHA (first commit)
    let master_branch_path = repository_dir
        .path()
        .join(".git")
        .join("refs")
        .join("heads")
        .join("master");
    let master_commit = std::fs::read_to_string(&master_branch_path)?;

    // Checkout using the commit SHA directly
    run_bit_command(repository_dir.path(), &["checkout", master_commit.trim()])
        .assert()
        .success();

    // Verify HEAD is updated to the master commit
    let head_path = repository_dir.path().join(".git").join("HEAD");
    let updated_head = std::fs::read_to_string(&head_path)?;
    assert_eq!(updated_head.trim(), master_commit.trim());

    // Verify workspace files match the first commit state
    let file1_content = std::fs::read_to_string(repository_dir.path().join("file1.txt"))?;
    assert_eq!(file1_content, "initial content 1");

    let file2_content = std::fs::read_to_string(repository_dir.path().join("file2.txt"))?;
    assert_eq!(file2_content, "initial content 2");

    // file3 should not exist (it was added in second commit)
    let file3_path = repository_dir.path().join("file3.txt");
    assert!(!file3_path.exists());

    Ok(())
}

#[rstest]
fn checkout_using_revision_syntax(
    repository_with_branches: TempDir,
) -> Result<(), Box<dyn std::error::Error>> {
    let repository_dir = repository_with_branches;

    // Get current HEAD
    let head_path = repository_dir.path().join(".git").join("HEAD");
    let current_head = std::fs::read_to_string(&head_path)?;

    // Checkout HEAD^ (parent of current commit)
    run_bit_command(repository_dir.path(), &["checkout", "HEAD^"])
        .assert()
        .success();

    // Verify HEAD changed
    let updated_head = std::fs::read_to_string(&head_path)?;
    assert_ne!(updated_head, current_head);

    // Verify workspace state matches the second commit
    let file1_content = std::fs::read_to_string(repository_dir.path().join("file1.txt"))?;
    assert_eq!(file1_content, "modified content 1");

    // file2 should be in initial state (not yet modified in second commit)
    let file2_content = std::fs::read_to_string(repository_dir.path().join("file2.txt"))?;
    assert_eq!(file2_content, "initial content 2");

    // file3 should exist (added in second commit)
    let file3_path = repository_dir.path().join("file3.txt");
    assert!(file3_path.exists());

    Ok(())
}

#[rstest]
fn checkout_using_alias_syntax(
    repository_with_branches: TempDir,
) -> Result<(), Box<dyn std::error::Error>> {
    let repository_dir = repository_with_branches;

    // Checkout @~2 (two commits before current HEAD using @ alias)
    run_bit_command(repository_dir.path(), &["checkout", "@~2"])
        .assert()
        .success();

    // Verify workspace state matches the first commit
    let file1_content = std::fs::read_to_string(repository_dir.path().join("file1.txt"))?;
    assert_eq!(file1_content, "initial content 1");

    let file2_content = std::fs::read_to_string(repository_dir.path().join("file2.txt"))?;
    assert_eq!(file2_content, "initial content 2");

    // file3 should not exist yet
    let file3_path = repository_dir.path().join("file3.txt");
    assert!(!file3_path.exists());

    Ok(())
}

#[rstest]
fn checkout_with_complex_file_and_directory_operations() -> Result<(), Box<dyn std::error::Error>> {
    crate::common::redirect_temp_dir();
    let repository_dir = TempDir::new().expect("Failed to create temp dir");

    run_bit_command(repository_dir.path(), &["init"])
        .assert()
        .success();

    // ============================================================
    // FIRST COMMIT: Initial state with files and directories
    // ============================================================

    // Create a regular file that will later be replaced with a directory
    let file_to_become_dir = FileSpec::new(
        repository_dir.path().join("will_become_dir.txt"),
        "I am a file".to_string(),
    );
    write_file(file_to_become_dir);

    // Create a directory with files that will later be replaced with a single file
    let dir_to_become_file = FileSpec::new(
        repository_dir
            .path()
            .join("will_become_file")
            .join("nested1.txt"),
        "nested content 1".to_string(),
    );
    write_file(dir_to_become_file);

    let dir_to_become_file2 = FileSpec::new(
        repository_dir
            .path()
            .join("will_become_file")
            .join("nested2.txt"),
        "nested content 2".to_string(),
    );
    write_file(dir_to_become_file2);

    // Create a file that will be deleted
    let file_to_delete = FileSpec::new(
        repository_dir.path().join("to_delete.txt"),
        "delete me".to_string(),
    );
    write_file(file_to_delete);

    // Create a directory that will be deleted
    let dir_to_delete = FileSpec::new(
        repository_dir
            .path()
            .join("dir_to_delete")
            .join("child.txt"),
        "in directory to delete".to_string(),
    );
    write_file(dir_to_delete);

    // Create a file that will be updated
    let file_to_update = FileSpec::new(
        repository_dir.path().join("to_update.txt"),
        "original content".to_string(),
    );
    write_file(file_to_update);

    // Create a nested directory structure that will be partially deleted
    let nested_dir = FileSpec::new(
        repository_dir
            .path()
            .join("nested")
            .join("level1")
            .join("level2.txt"),
        "deep nested".to_string(),
    );
    write_file(nested_dir);

    run_bit_command(repository_dir.path(), &["add", "."])
        .assert()
        .success();

    bit_commit(repository_dir.path(), "First commit - initial state")
        .assert()
        .success();

    // Create branch pointing to first commit
    run_bit_command(repository_dir.path(), &["branch", "initial-state"])
        .assert()
        .success();

    // ============================================================
    // SECOND COMMIT: Complex transformations
    // ============================================================

    // Replace file with directory (will_become_dir.txt -> will_become_dir/)
    std::fs::remove_file(repository_dir.path().join("will_become_dir.txt"))?;
    let new_dir = FileSpec::new(
        repository_dir
            .path()
            .join("will_become_dir")
            .join("now_a_dir.txt"),
        "I am now in a directory".to_string(),
    );
    write_file(new_dir);

    let new_dir2 = FileSpec::new(
        repository_dir
            .path()
            .join("will_become_dir")
            .join("another.txt"),
        "another file in dir".to_string(),
    );
    write_file(new_dir2);

    // Replace directory with file (will_become_file/ -> will_become_file.txt)
    std::fs::remove_dir_all(repository_dir.path().join("will_become_file"))?;
    let new_file = FileSpec::new(
        repository_dir.path().join("will_become_file.txt"),
        "I am now a file".to_string(),
    );
    write_file(new_file);

    // Delete file
    std::fs::remove_file(repository_dir.path().join("to_delete.txt"))?;

    // Delete directory
    std::fs::remove_dir_all(repository_dir.path().join("dir_to_delete"))?;

    // Update file content
    let updated_file = FileSpec::new(
        repository_dir.path().join("to_update.txt"),
        "updated content - version 2".to_string(),
    );
    write_file(updated_file);

    // Add new file
    let new_file_added = FileSpec::new(
        repository_dir.path().join("newly_added.txt"),
        "I am new".to_string(),
    );
    write_file(new_file_added);

    // Add new nested directory
    let new_nested = FileSpec::new(
        repository_dir
            .path()
            .join("new_dir")
            .join("subdir")
            .join("file.txt"),
        "new nested structure".to_string(),
    );
    write_file(new_nested);

    // Delete part of nested structure
    std::fs::remove_dir_all(repository_dir.path().join("nested"))?;

    run_bit_command(repository_dir.path(), &["add", "."])
        .assert()
        .success();

    bit_commit(
        repository_dir.path(),
        "Second commit - complex transformations",
    )
    .assert()
    .success();

    // ============================================================
    // VERIFICATION: Check current state (second commit)
    // ============================================================

    // Verify file was replaced with directory
    assert!(!repository_dir.path().join("will_become_dir.txt").exists());
    assert!(repository_dir.path().join("will_become_dir").is_dir());
    assert!(
        repository_dir
            .path()
            .join("will_become_dir")
            .join("now_a_dir.txt")
            .exists()
    );

    // Verify directory was replaced with file
    assert!(!repository_dir.path().join("will_become_file").exists());
    assert!(repository_dir.path().join("will_become_file.txt").is_file());

    // Verify deletions
    assert!(!repository_dir.path().join("to_delete.txt").exists());
    assert!(!repository_dir.path().join("dir_to_delete").exists());

    // Verify update
    let updated_content = std::fs::read_to_string(repository_dir.path().join("to_update.txt"))?;
    assert_eq!(updated_content, "updated content - version 2");

    // Verify additions
    assert!(repository_dir.path().join("newly_added.txt").exists());
    assert!(
        repository_dir
            .path()
            .join("new_dir")
            .join("subdir")
            .join("file.txt")
            .exists()
    );

    // Verify nested deletion
    assert!(!repository_dir.path().join("nested").exists());

    // ============================================================
    // CHECKOUT: Go back to first commit
    // ============================================================

    run_bit_command(repository_dir.path(), &["checkout", "initial-state"])
        .assert()
        .success();

    // ============================================================
    // VERIFICATION: Check restored state (first commit)
    // ============================================================

    // File should be restored (was replaced with dir in second commit)
    assert!(repository_dir.path().join("will_become_dir.txt").is_file());
    assert!(!repository_dir.path().join("will_become_dir").exists());
    let restored_file_content =
        std::fs::read_to_string(repository_dir.path().join("will_become_dir.txt"))?;
    assert_eq!(restored_file_content, "I am a file");

    // Directory should be restored (was replaced with file in second commit)
    assert!(!repository_dir.path().join("will_become_file.txt").exists());
    assert!(repository_dir.path().join("will_become_file").is_dir());
    assert!(
        repository_dir
            .path()
            .join("will_become_file")
            .join("nested1.txt")
            .exists()
    );
    assert!(
        repository_dir
            .path()
            .join("will_become_file")
            .join("nested2.txt")
            .exists()
    );
    let restored_nested1 = std::fs::read_to_string(
        repository_dir
            .path()
            .join("will_become_file")
            .join("nested1.txt"),
    )?;
    assert_eq!(restored_nested1, "nested content 1");

    // Deleted file should be restored
    assert!(repository_dir.path().join("to_delete.txt").is_file());
    let restored_deleted_file =
        std::fs::read_to_string(repository_dir.path().join("to_delete.txt"))?;
    assert_eq!(restored_deleted_file, "delete me");

    // Deleted directory should be restored
    assert!(repository_dir.path().join("dir_to_delete").is_dir());
    assert!(
        repository_dir
            .path()
            .join("dir_to_delete")
            .join("child.txt")
            .exists()
    );
    let restored_dir_file = std::fs::read_to_string(
        repository_dir
            .path()
            .join("dir_to_delete")
            .join("child.txt"),
    )?;
    assert_eq!(restored_dir_file, "in directory to delete");

    // Updated file should have original content
    let original_content = std::fs::read_to_string(repository_dir.path().join("to_update.txt"))?;
    assert_eq!(original_content, "original content");

    // Files added in second commit should not exist
    assert!(!repository_dir.path().join("newly_added.txt").exists());
    assert!(!repository_dir.path().join("new_dir").exists());

    // Nested structure should be restored
    assert!(repository_dir.path().join("nested").is_dir());
    assert!(
        repository_dir
            .path()
            .join("nested")
            .join("level1")
            .join("level2.txt")
            .exists()
    );
    let restored_nested = std::fs::read_to_string(
        repository_dir
            .path()
            .join("nested")
            .join("level1")
            .join("level2.txt"),
    )?;
    assert_eq!(restored_nested, "deep nested");

    Ok(())
}
