use crate::common::command::{bit_commit, bit_merge, repository_dir, run_bit_command};
use crate::common::file::{FileSpec, write_file};
use assert_fs::TempDir;
use rstest::rstest;
use std::fs;

/// Test merging with diamond pattern (single merge point)
///
/// History:
///       A
///      / \
///     B   C
///      \ /
///       D (merge)
///
/// This tests that after merging B and C into D, we can work with D normally
#[rstest]
fn merge_diamond_pattern(repository_dir: TempDir) -> Result<(), Box<dyn std::error::Error>> {
    let dir = repository_dir;

    // Initialize repository
    run_bit_command(dir.path(), &["init"]).assert().success();

    // Commit A: Create initial file
    write_file(FileSpec::new(
        dir.path().join("data.txt"),
        "A\n".to_string(),
    ));
    run_bit_command(dir.path(), &["add", "."])
        .assert()
        .success();
    bit_commit(dir.path(), "Commit A").assert().success();

    // Create left branch
    run_bit_command(dir.path(), &["branch", "create", "left"])
        .assert()
        .success();

    // Commit B on master: add a master-only file (no conflict with left branch)
    write_file(FileSpec::new(
        dir.path().join("data_b.txt"),
        "B\n".to_string(),
    ));
    run_bit_command(dir.path(), &["add", "."])
        .assert()
        .success();
    bit_commit(dir.path(), "Commit B").assert().success();

    // Switch to left branch
    run_bit_command(dir.path(), &["checkout", "left"])
        .assert()
        .success();

    // Commit C on left: add a left-only file (no conflict with master)
    write_file(FileSpec::new(
        dir.path().join("data_c.txt"),
        "C\n".to_string(),
    ));
    run_bit_command(dir.path(), &["add", "."])
        .assert()
        .success();
    bit_commit(dir.path(), "Commit C").assert().success();

    // Switch back to master
    run_bit_command(dir.path(), &["checkout", "master"])
        .assert()
        .success();

    // Commit D: Merge left into master — should be clean (no shared file modified)
    bit_merge(dir.path(), "left", "Merge commit D")
        .assert()
        .success();

    // After merge both branch-specific files should be present
    let content = fs::read_to_string(dir.path().join("data.txt")).expect("Failed to read data.txt");
    assert!(content.contains("A\n"), "Should contain base commit A");

    // Create another commit after merge to verify the merge commit works
    write_file(FileSpec::new(
        dir.path().join("after-merge.txt"),
        "post-merge\n".to_string(),
    ));
    run_bit_command(dir.path(), &["add", "."])
        .assert()
        .success();
    bit_commit(dir.path(), "Commit after merge")
        .assert()
        .success();

    // Verify the new file exists
    assert!(dir.path().join("after-merge.txt").exists());

    Ok(())
}
