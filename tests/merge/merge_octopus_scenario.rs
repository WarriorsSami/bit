use crate::common::command::{bit_commit, bit_merge, repository_dir, run_bit_command};
use crate::common::file::{FileSpec, write_file};
use assert_fs::TempDir;
use rstest::rstest;
use std::fs;

/// Test merging with octopus-like scenario (multiple parents)
///
/// History:
///       A
///      /|\
///     B C D
///      \|/
///       E (octopus merge of B, C, D)
///
/// We simulate this by doing sequential merges
#[rstest]
fn merge_octopus_scenario(repository_dir: TempDir) -> Result<(), Box<dyn std::error::Error>> {
    let dir = repository_dir;

    // Initialize repository
    run_bit_command(dir.path(), &["init"]).assert().success();

    // Commit A: Base commit
    write_file(FileSpec::new(
        dir.path().join("base.txt"),
        "base\n".to_string(),
    ));
    run_bit_command(dir.path(), &["add", "."])
        .assert()
        .success();
    bit_commit(dir.path(), "Commit A").assert().success();

    // Create branches
    run_bit_command(dir.path(), &["branch", "create", "branch-b"])
        .assert()
        .success();
    run_bit_command(dir.path(), &["branch", "create", "branch-c"])
        .assert()
        .success();
    run_bit_command(dir.path(), &["branch", "create", "branch-d"])
        .assert()
        .success();

    // Commit B on branch-b
    run_bit_command(dir.path(), &["checkout", "branch-b"])
        .assert()
        .success();
    write_file(FileSpec::new(
        dir.path().join("b.txt"),
        "B changes\n".to_string(),
    ));
    run_bit_command(dir.path(), &["add", "."])
        .assert()
        .success();
    bit_commit(dir.path(), "Commit B").assert().success();

    // Commit C on branch-c
    run_bit_command(dir.path(), &["checkout", "branch-c"])
        .assert()
        .success();
    write_file(FileSpec::new(
        dir.path().join("c.txt"),
        "C changes\n".to_string(),
    ));
    run_bit_command(dir.path(), &["add", "."])
        .assert()
        .success();
    bit_commit(dir.path(), "Commit C").assert().success();

    // Commit D on branch-d
    run_bit_command(dir.path(), &["checkout", "branch-d"])
        .assert()
        .success();
    write_file(FileSpec::new(
        dir.path().join("d.txt"),
        "D changes\n".to_string(),
    ));
    run_bit_command(dir.path(), &["add", "."])
        .assert()
        .success();
    bit_commit(dir.path(), "Commit D").assert().success();

    // Switch to master for octopus merge
    run_bit_command(dir.path(), &["checkout", "master"])
        .assert()
        .success();

    // First merge: merge branch-b
    bit_merge(dir.path(), "branch-b", "Merge B")
        .assert()
        .success();

    // Second merge: merge branch-c
    bit_merge(dir.path(), "branch-c", "Merge C")
        .assert()
        .success();

    // Third merge: merge branch-d (completing the octopus-like pattern)
    bit_merge(dir.path(), "branch-d", "Merge D - octopus complete")
        .assert()
        .success();

    // Verify all files from all branches exist
    assert!(dir.path().join("base.txt").exists());
    assert!(dir.path().join("b.txt").exists());
    assert!(dir.path().join("c.txt").exists());
    assert!(dir.path().join("d.txt").exists());

    // Verify content
    let base_content =
        fs::read_to_string(dir.path().join("base.txt")).expect("Failed to read base.txt");
    assert_eq!(base_content, "base\n");

    let b_content = fs::read_to_string(dir.path().join("b.txt")).expect("Failed to read b.txt");
    assert_eq!(b_content, "B changes\n");

    let c_content = fs::read_to_string(dir.path().join("c.txt")).expect("Failed to read c.txt");
    assert_eq!(c_content, "C changes\n");

    let d_content = fs::read_to_string(dir.path().join("d.txt")).expect("Failed to read d.txt");
    assert_eq!(d_content, "D changes\n");

    Ok(())
}
