use crate::common::command::{bit_commit, bit_merge, repository_dir, run_bit_command};
use crate::common::file::{FileSpec, delete_path, write_file};
use assert_fs::TempDir;
use rstest::rstest;

/// Long-format status shows "deleted by them" when we modified a file that they deleted.
///
/// History:
///   A: f.txt = "original\n"
///   B (master):  f.txt = "modified\n"   (we modified)
///   C (topic):   f.txt deleted          (they deleted)
///
/// After merging topic into master (fails with conflict), status must show
/// "deleted by them: f.txt" in "Unmerged paths".
#[rstest]
fn report_deleted_by_them_conflict_long(
    repository_dir: TempDir,
) -> Result<(), Box<dyn std::error::Error>> {
    let dir = repository_dir;

    run_bit_command(dir.path(), &["init"]).assert().success();

    write_file(FileSpec::new(dir.path().join("f.txt"), "original\n".into()));
    run_bit_command(dir.path(), &["add", "."])
        .assert()
        .success();
    bit_commit(dir.path(), "Commit A").assert().success();

    run_bit_command(dir.path(), &["branch", "create", "topic"])
        .assert()
        .success();

    // Modify f.txt on master
    write_file(FileSpec::new(dir.path().join("f.txt"), "modified\n".into()));
    run_bit_command(dir.path(), &["add", "."])
        .assert()
        .success();
    bit_commit(dir.path(), "Commit B - modify")
        .assert()
        .success();

    // Delete f.txt on topic
    run_bit_command(dir.path(), &["checkout", "topic"])
        .assert()
        .success();
    delete_path(&dir.path().join("f.txt"));
    run_bit_command(dir.path(), &["add", "."])
        .assert()
        .success();
    bit_commit(dir.path(), "Commit C - delete")
        .assert()
        .success();

    run_bit_command(dir.path(), &["checkout", "master"])
        .assert()
        .success();

    // Merge fails due to conflict
    bit_merge(dir.path(), "topic", "merge topic")
        .assert()
        .failure();

    let output = run_bit_command(dir.path(), &["status"]).assert().success();
    let stdout = String::from_utf8(output.get_output().stdout.clone())?;

    assert!(
        stdout.contains("Unmerged paths"),
        "Expected 'Unmerged paths' section in:\n{}",
        stdout
    );
    assert!(
        stdout.contains("deleted by them"),
        "Expected 'deleted by them' label in:\n{}",
        stdout
    );
    assert!(
        stdout.contains("f.txt"),
        "Expected 'f.txt' in unmerged section:\n{}",
        stdout
    );

    Ok(())
}
