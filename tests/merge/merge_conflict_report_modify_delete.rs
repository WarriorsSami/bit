use crate::common::command::{bit_commit, bit_merge, repository_dir, run_bit_command};
use crate::common::file::{FileSpec, delete_path, write_file};
use assert_fs::TempDir;
use rstest::rstest;

/// Test that a modify/delete conflict reports which side deleted and which modified
///
/// History:
///   A: file.txt = "original\n"
///   B (master):  file.txt = "modified\n"
///   C (feature): file.txt deleted
///
/// Expected stdout:
///   CONFLICT (modify/delete): file.txt deleted in feature and modified in HEAD
#[rstest]
fn merge_conflict_report_modify_delete(
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

    // Merge must fail with a conflict
    let output = bit_merge(dir.path(), "feature", "modify/delete")
        .assert()
        .failure();
    let stdout = String::from_utf8(output.get_output().stdout.clone())?;

    assert!(
        stdout
            .contains("CONFLICT (modify/delete): file.txt deleted in feature and modified in HEAD"),
        "Expected 'CONFLICT (modify/delete): file.txt deleted in feature and modified in HEAD' in stdout, got:\n{}",
        stdout
    );

    Ok(())
}
