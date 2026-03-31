use crate::common::command::{bit_commit, bit_merge, repository_dir, run_bit_command};
use crate::common::file::{FileSpec, write_file};
use assert_fs::TempDir;
use rstest::rstest;

/// Test that a directory/file conflict reports the collision and the rename applied
///
/// This is the inverse of the file/directory case: HEAD has a directory at f.txt/
/// and feature introduces a regular file at f.txt. The incoming file gets displaced.
///
/// History:
///   A: anchor.txt only
///   B (master):  f.txt/g.txt — a directory named f.txt containing g.txt
///   C (feature): f.txt as a regular file ("file content\n")
///
/// Expected stdout:
///   CONFLICT (directory/file): There is a directory with name f.txt in HEAD. Adding f.txt as f.txt~feature
#[rstest]
fn merge_conflict_report_directory_file(
    repository_dir: TempDir,
) -> Result<(), Box<dyn std::error::Error>> {
    let dir = repository_dir;

    run_bit_command(dir.path(), &["init"]).assert().success();

    // Commit A: anchor only
    write_file(FileSpec::new(
        dir.path().join("anchor.txt"),
        "anchor\n".into(),
    ));
    run_bit_command(dir.path(), &["add", "."])
        .assert()
        .success();
    bit_commit(dir.path(), "Commit A").assert().success();

    // Create feature branch at A
    run_bit_command(dir.path(), &["branch", "create", "feature"])
        .assert()
        .success();

    // Commit C on feature: add f.txt as a regular file.
    // Do this before committing on master so the return checkout (feature → master)
    // only needs to delete a plain file, not convert a file into a directory.
    run_bit_command(dir.path(), &["checkout", "feature"])
        .assert()
        .success();
    write_file(FileSpec::new(
        dir.path().join("f.txt"),
        "file content\n".into(),
    ));
    run_bit_command(dir.path(), &["add", "."])
        .assert()
        .success();
    bit_commit(dir.path(), "Commit C - f.txt as file")
        .assert()
        .success();

    // Return to master while workspace still only has anchor.txt + f.txt (plain file).
    // At this point master == A (anchor.txt only), so checkout simply removes f.txt.
    run_bit_command(dir.path(), &["checkout", "master"])
        .assert()
        .success();

    // Commit B on master: add f.txt/g.txt (directory named f.txt)
    write_file(FileSpec::new(
        dir.path().join("f.txt").join("g.txt"),
        "dir content\n".into(),
    ));
    run_bit_command(dir.path(), &["add", "."])
        .assert()
        .success();
    bit_commit(dir.path(), "Commit B - f.txt as directory")
        .assert()
        .success();

    // Merge must fail due to path-type collision
    let output = bit_merge(dir.path(), "feature", "dir-file conflict")
        .assert()
        .failure();
    let stdout = String::from_utf8(output.get_output().stdout.clone())?;

    assert!(
        stdout.contains("CONFLICT (directory/file): There is a directory with name f.txt in HEAD. Adding f.txt as f.txt~feature"),
        "Expected directory/file conflict message in stdout, got:\n{}",
        stdout
    );

    Ok(())
}
