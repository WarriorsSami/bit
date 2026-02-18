use crate::common::command::{bit_commit, bit_merge, repository_dir, run_bit_command};
use crate::common::file::{FileSpec, write_file};
use assert_fs::TempDir;
use rstest::rstest;
use std::fs;

/// Test merging with simple divergent branches
///
/// History:
///       A (base)
///      / \
///     B   C
///     |   |
///   master  feature
///
/// Expected: Merge commit combining B and C with A as common ancestor
/// Final content should include changes from both branches
#[rstest]
fn merge_simple_divergence(repository_dir: TempDir) -> Result<(), Box<dyn std::error::Error>> {
    let dir = repository_dir;

    // Initialize repository
    run_bit_command(dir.path(), &["init"]).assert().success();

    // Commit A: Create initial files
    write_file(FileSpec::new(
        dir.path().join("base.txt"),
        "base content\n".to_string(),
    ));
    write_file(FileSpec::new(
        dir.path().join("left.txt"),
        "initial\n".to_string(),
    ));
    write_file(FileSpec::new(
        dir.path().join("right.txt"),
        "initial\n".to_string(),
    ));
    run_bit_command(dir.path(), &["add", "."])
        .assert()
        .success();
    bit_commit(dir.path(), "Commit A - base").assert().success();

    // Create feature branch at A
    run_bit_command(dir.path(), &["branch", "create", "feature"])
        .assert()
        .success();

    // Commit B on master: Modify left.txt
    write_file(FileSpec::new(
        dir.path().join("left.txt"),
        "initial\nmaster change\n".to_string(),
    ));
    run_bit_command(dir.path(), &["add", "."])
        .assert()
        .success();
    bit_commit(dir.path(), "Commit B - master changes")
        .assert()
        .success();

    // Switch to feature branch
    run_bit_command(dir.path(), &["checkout", "feature"])
        .assert()
        .success();

    // Commit C on feature: Modify right.txt
    write_file(FileSpec::new(
        dir.path().join("right.txt"),
        "initial\nfeature change\n".to_string(),
    ));
    run_bit_command(dir.path(), &["add", "."])
        .assert()
        .success();
    bit_commit(dir.path(), "Commit C - feature changes")
        .assert()
        .success();

    // Switch back to master
    run_bit_command(dir.path(), &["checkout", "master"])
        .assert()
        .success();

    // Merge feature into master
    bit_merge(dir.path(), "feature", "Merge feature into master")
        .assert()
        .success();

    // Verify final file contents - should have both changes
    let base_content =
        fs::read_to_string(dir.path().join("base.txt")).expect("Failed to read base.txt");
    assert_eq!(base_content, "base content\n");

    let left_content =
        fs::read_to_string(dir.path().join("left.txt")).expect("Failed to read left.txt");
    assert_eq!(left_content, "initial\nmaster change\n");

    let right_content =
        fs::read_to_string(dir.path().join("right.txt")).expect("Failed to read right.txt");
    assert_eq!(right_content, "initial\nfeature change\n");

    Ok(())
}
