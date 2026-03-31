use crate::common::command::{bit_commit, bit_merge, repository_dir, run_bit_command};
use crate::common::file::{FileSpec, delete_path, write_file};
use assert_fs::TempDir;
use rstest::rstest;

/// Test that a delete/modify conflict reports which side deleted and which modified
///
/// This is the inverse of the modify/delete case: HEAD deleted the file,
/// feature modified it.
///
/// History:
///   A: file.txt = "original\n"
///   B (master):  file.txt deleted
///   C (feature): file.txt = "modified\n"
///
/// Expected stdout:
///   CONFLICT (delete/modify): file.txt deleted in HEAD and modified in feature
#[rstest]
fn merge_conflict_report_delete_modify(
    repository_dir: TempDir,
) -> Result<(), Box<dyn std::error::Error>> {
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

    // Commit B on master: delete file.txt
    delete_path(&dir.path().join("file.txt"));
    run_bit_command(dir.path(), &["add", "."])
        .assert()
        .success();
    bit_commit(dir.path(), "Commit B - delete")
        .assert()
        .success();

    // Commit C on feature: modify file.txt
    run_bit_command(dir.path(), &["checkout", "feature"])
        .assert()
        .success();
    write_file(FileSpec::new(
        dir.path().join("file.txt"),
        "modified\n".into(),
    ));
    run_bit_command(dir.path(), &["add", "."])
        .assert()
        .success();
    bit_commit(dir.path(), "Commit C - modify")
        .assert()
        .success();

    run_bit_command(dir.path(), &["checkout", "master"])
        .assert()
        .success();

    // Merge must fail with a conflict
    let output = bit_merge(dir.path(), "feature", "delete/modify")
        .assert()
        .failure();
    let stdout = String::from_utf8(output.get_output().stdout.clone())?;

    assert!(
        stdout
            .contains("CONFLICT (delete/modify): file.txt deleted in HEAD and modified in feature"),
        "Expected 'CONFLICT (delete/modify): file.txt deleted in HEAD and modified in feature' in stdout, got:\n{}",
        stdout
    );

    Ok(())
}
