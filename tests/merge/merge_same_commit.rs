use crate::common::command::{bit_commit, bit_merge, repository_dir, run_bit_command};
use crate::common::file::{FileSpec, write_file};
use assert_fs::TempDir;
use rstest::rstest;
use std::fs;

/// Test merging a branch that points to the same commit as HEAD
/// This is the simplest case and should complete quickly
#[rstest]
fn merge_same_commit(repository_dir: TempDir) -> Result<(), Box<dyn std::error::Error>> {
    // Initialize repository
    run_bit_command(repository_dir.path(), &["init"])
        .assert()
        .success();

    // Create initial commit
    write_file(FileSpec::new(
        repository_dir.path().join("file.txt"),
        "content\n".to_string(),
    ));
    run_bit_command(repository_dir.path(), &["add", "."])
        .assert()
        .success();
    bit_commit(repository_dir.path(), "Initial commit")
        .assert()
        .success();

    // Create a branch pointing to the same commit
    run_bit_command(repository_dir.path(), &["branch", "create", "same"])
        .assert()
        .success();

    // Merge the branch (should be a no-op)
    bit_merge(repository_dir.path(), "same", "Merge same commit")
        .assert()
        .success();

    // Verify file content is unchanged
    let content =
        fs::read_to_string(repository_dir.path().join("file.txt")).expect("Failed to read file");
    assert_eq!(content, "content\n");

    Ok(())
}
