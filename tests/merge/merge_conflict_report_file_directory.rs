use crate::common::command::{bit_commit, bit_merge, repository_dir, run_bit_command};
use crate::common::file::{FileSpec, write_file};
use assert_fs::TempDir;
use rstest::rstest;

/// Test that a file/directory conflict reports the collision and the rename applied
///
/// History:
///   A: anchor.txt only
///   B (master):  f.txt as a regular file ("file content\n")
///   C (feature): f.txt/g.txt — a directory named f.txt containing g.txt
///
/// Expected stdout:
///   CONFLICT (file/directory): There is a directory with name f.txt in feature. Adding f.txt as f.txt~HEAD
#[rstest]
fn merge_conflict_report_file_directory(
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

    // Commit B on master: add f.txt as a regular file
    write_file(FileSpec::new(
        dir.path().join("f.txt"),
        "file content\n".into(),
    ));
    run_bit_command(dir.path(), &["add", "."])
        .assert()
        .success();
    bit_commit(dir.path(), "Commit B - f.txt as file")
        .assert()
        .success();

    // Commit C on feature: add f.txt/g.txt (directory with same name as the file)
    run_bit_command(dir.path(), &["checkout", "feature"])
        .assert()
        .success();
    write_file(FileSpec::new(
        dir.path().join("f.txt").join("g.txt"),
        "dir content\n".into(),
    ));
    run_bit_command(dir.path(), &["add", "."])
        .assert()
        .success();
    bit_commit(dir.path(), "Commit C - f.txt as directory")
        .assert()
        .success();

    run_bit_command(dir.path(), &["checkout", "master"])
        .assert()
        .success();

    // Merge must fail due to path-type collision
    let output = bit_merge(dir.path(), "feature", "file-dir conflict")
        .assert()
        .failure();
    let stdout = String::from_utf8(output.get_output().stdout.clone())?;

    assert!(
        stdout.contains("CONFLICT (file/directory): There is a directory with name f.txt in feature. Adding f.txt as f.txt~HEAD"),
        "Expected file/directory conflict message in stdout, got:\n{}",
        stdout
    );

    Ok(())
}
