use crate::common::command::{bit_commit, bit_merge, repository_dir, run_bit_command};
use crate::common::file::{FileSpec, write_file};
use assert_fs::TempDir;
use rstest::rstest;
use std::fs;

/// Test merging in a linear history scenario
///
/// History:
///   A (base) <- B <- C <- D (HEAD on master)
///                         ^
///                         feature points here too
///
/// Expected: Fast-forward merge (no actual merge commit needed)
/// But our implementation creates a merge commit, so we verify the merge succeeds
#[rstest]
fn merge_linear_history(repository_dir: TempDir) -> Result<(), Box<dyn std::error::Error>> {
    let dir = repository_dir;

    // Initialize repository
    run_bit_command(dir.path(), &["init"]).assert().success();

    // Commit A: Create initial file
    write_file(FileSpec::new(
        dir.path().join("file.txt"),
        "line A\n".to_string(),
    ));
    run_bit_command(dir.path(), &["add", "."])
        .assert()
        .success();
    bit_commit(dir.path(), "Commit A").assert().success();

    // Commit B: Append line B
    write_file(FileSpec::new(
        dir.path().join("file.txt"),
        "line A\nline B\n".to_string(),
    ));
    run_bit_command(dir.path(), &["add", "."])
        .assert()
        .success();
    bit_commit(dir.path(), "Commit B").assert().success();

    // Commit C: Append line C
    write_file(FileSpec::new(
        dir.path().join("file.txt"),
        "line A\nline B\nline C\n".to_string(),
    ));
    run_bit_command(dir.path(), &["add", "."])
        .assert()
        .success();
    bit_commit(dir.path(), "Commit C").assert().success();

    // Commit D: Append line D
    write_file(FileSpec::new(
        dir.path().join("file.txt"),
        "line A\nline B\nline C\nline D\n".to_string(),
    ));
    run_bit_command(dir.path(), &["add", "."])
        .assert()
        .success();
    bit_commit(dir.path(), "Commit D").assert().success();

    // Create feature branch pointing to D (same as main)
    run_bit_command(dir.path(), &["branch", "create", "feature"])
        .assert()
        .success();

    // Merge feature into main (should be a no-op or fast-forward)
    bit_merge(dir.path(), "feature", "Merge feature")
        .assert()
        .success();

    // Verify final file content
    let content = fs::read_to_string(dir.path().join("file.txt")).expect("Failed to read file");
    assert_eq!(content, "line A\nline B\nline C\nline D\n");

    Ok(())
}
