use crate::common::command::{bit_commit, bit_merge, repository_dir, run_bit_command};
use crate::common::file::{FileSpec, delete_path, write_file};
use assert_fs::TempDir;
use rstest::rstest;
use std::fs;

/// Test that master modifying a file that feature deleted is treated as a conflict
///
/// History:
///   A: file.txt = "original\n"
///   B (master):  file.txt = "modified\n"
///   C (feature): file.txt deleted
///
/// Expected: non-zero exit, error names file.txt, modified version preserved in workspace
#[rstest]
fn merge_modify_delete_conflict(repository_dir: TempDir) -> Result<(), Box<dyn std::error::Error>> {
    let dir = repository_dir;

    run_bit_command(dir.path(), &["init"]).assert().success();

    // Commit A: original file
    write_file(FileSpec::new(
        dir.path().join("file.txt"),
        "original\n".into(),
    ));
    run_bit_command(dir.path(), &["add", "."])
        .assert()
        .success();
    bit_commit(dir.path(), "Commit A").assert().success();

    // Create feature branch at A
    run_bit_command(dir.path(), &["branch", "create", "feature"])
        .assert()
        .success();

    // Commit B on master: modify file.txt
    write_file(FileSpec::new(
        dir.path().join("file.txt"),
        "modified\n".into(),
    ));
    run_bit_command(dir.path(), &["add", "."])
        .assert()
        .success();
    bit_commit(dir.path(), "Commit B - modify")
        .assert()
        .success();

    // Commit C on feature: delete file.txt
    run_bit_command(dir.path(), &["checkout", "feature"])
        .assert()
        .success();
    delete_path(&dir.path().join("file.txt"));
    run_bit_command(dir.path(), &["add", "."])
        .assert()
        .success();
    bit_commit(dir.path(), "Commit C - delete")
        .assert()
        .success();

    run_bit_command(dir.path(), &["checkout", "master"])
        .assert()
        .success();

    // Merge must fail
    let output = bit_merge(dir.path(), "feature", "modify/delete")
        .assert()
        .failure();
    let stderr = String::from_utf8(output.get_output().stderr.clone())?;

    assert!(
        stderr.contains("file.txt"),
        "Error output should name the conflicting file, got:\n{}",
        stderr
    );

    // The modified version must be preserved — not deleted
    assert!(
        dir.path().join("file.txt").exists(),
        "file.txt should be preserved in workspace on modify/delete conflict"
    );
    let content = fs::read_to_string(dir.path().join("file.txt"))?;
    assert_eq!(
        content, "modified\n",
        "file.txt should contain master's modification, not be lost"
    );

    Ok(())
}
