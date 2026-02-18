use crate::common::command::{bit_commit, bit_merge, repository_dir, run_bit_command};
use crate::common::file::{FileSpec, write_file};
use assert_fs::TempDir;
use rstest::rstest;
use std::fs;

/// Test merging with criss-cross pattern (multiple best common ancestors)
///
/// History:
///       A
///      / \
///     B   C
///     |\ /|
///     | X |
///     |/ \|
///     D   E
///     |   |
///     F   G
///
/// When merging F and G, both D and E are best common ancestors
#[rstest]
fn merge_criss_cross(repository_dir: TempDir) -> Result<(), Box<dyn std::error::Error>> {
    let dir = repository_dir;

    // Initialize repository
    run_bit_command(dir.path(), &["init"]).assert().success();

    // Commit A: Base commit
    write_file(FileSpec::new(
        dir.path().join("file1.txt"),
        "A\n".to_string(),
    ));
    run_bit_command(dir.path(), &["add", "."])
        .assert()
        .success();
    bit_commit(dir.path(), "Commit A").assert().success();

    // Create branch-left
    run_bit_command(dir.path(), &["branch", "create", "branch-left"])
        .assert()
        .success();

    // Commit B on master
    write_file(FileSpec::new(
        dir.path().join("file1.txt"),
        "A\nB\n".to_string(),
    ));
    run_bit_command(dir.path(), &["add", "."])
        .assert()
        .success();
    bit_commit(dir.path(), "Commit B").assert().success();

    // Switch to branch-left
    run_bit_command(dir.path(), &["checkout", "branch-left"])
        .assert()
        .success();

    // Commit C on branch-left
    write_file(FileSpec::new(
        dir.path().join("file1.txt"),
        "A\nC\n".to_string(),
    ));
    run_bit_command(dir.path(), &["add", "."])
        .assert()
        .success();
    bit_commit(dir.path(), "Commit C").assert().success();

    // Create branch-right from branch-left
    run_bit_command(dir.path(), &["branch", "create", "branch-right"])
        .assert()
        .success();

    // Switch back to master
    run_bit_command(dir.path(), &["checkout", "master"])
        .assert()
        .success();

    // Commit D: Merge branch-left into master (B + C)
    bit_merge(dir.path(), "branch-left", "Commit D - merge B and C")
        .assert()
        .success();

    // Switch to branch-right
    run_bit_command(dir.path(), &["checkout", "branch-right"])
        .assert()
        .success();

    // Commit E: Merge master (which now has D) into branch-right (C + B via D)
    bit_merge(dir.path(), "master", "Commit E - merge C and B")
        .assert()
        .success();

    // Switch back to master
    run_bit_command(dir.path(), &["checkout", "master"])
        .assert()
        .success();

    // Commit F: Add a change on master
    write_file(FileSpec::new(
        dir.path().join("file2.txt"),
        "F\n".to_string(),
    ));
    run_bit_command(dir.path(), &["add", "."])
        .assert()
        .success();
    bit_commit(dir.path(), "Commit F").assert().success();

    // Switch to branch-right
    run_bit_command(dir.path(), &["checkout", "branch-right"])
        .assert()
        .success();

    // Commit G: Add a change on branch-right
    write_file(FileSpec::new(
        dir.path().join("file3.txt"),
        "G\n".to_string(),
    ));
    run_bit_command(dir.path(), &["add", "."])
        .assert()
        .success();
    bit_commit(dir.path(), "Commit G").assert().success();

    // Switch back to master
    run_bit_command(dir.path(), &["checkout", "master"])
        .assert()
        .success();

    // Final merge: Merge G into F (both D and E are BCAs)
    bit_merge(dir.path(), "branch-right", "Merge criss-cross")
        .assert()
        .success();

    // Verify all files exist after merge
    assert!(dir.path().join("file1.txt").exists());
    assert!(dir.path().join("file2.txt").exists());
    assert!(dir.path().join("file3.txt").exists());

    let file2_content =
        fs::read_to_string(dir.path().join("file2.txt")).expect("Failed to read file2.txt");
    assert_eq!(file2_content, "F\n");

    let file3_content =
        fs::read_to_string(dir.path().join("file3.txt")).expect("Failed to read file3.txt");
    assert_eq!(file3_content, "G\n");

    Ok(())
}
