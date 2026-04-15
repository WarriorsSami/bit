use crate::common::command::{bit_commit, bit_merge, repository_dir, run_bit_command};
use crate::common::file::{FileSpec, write_file};
use assert_fs::TempDir;
use rstest::rstest;

/// Long-format status shows "both modified" when both branches edit the same file.
///
/// History:
///   A: f.txt = "base\n"
///   B (master):  f.txt = "master change\n"
///   C (topic):   f.txt = "topic change\n"
///
/// After merging topic into master (fails), status must show the "Unmerged paths"
/// section with "both modified:   f.txt".
#[rstest]
fn report_both_modified_conflict_long(
    repository_dir: TempDir,
) -> Result<(), Box<dyn std::error::Error>> {
    let dir = repository_dir;

    run_bit_command(dir.path(), &["init"]).assert().success();

    write_file(FileSpec::new(dir.path().join("f.txt"), "base\n".into()));
    run_bit_command(dir.path(), &["add", "."])
        .assert()
        .success();
    bit_commit(dir.path(), "Commit A").assert().success();

    run_bit_command(dir.path(), &["branch", "create", "topic"])
        .assert()
        .success();

    // Modify f.txt on master
    write_file(FileSpec::new(
        dir.path().join("f.txt"),
        "master change\n".into(),
    ));
    run_bit_command(dir.path(), &["add", "."])
        .assert()
        .success();
    bit_commit(dir.path(), "Commit B - master")
        .assert()
        .success();

    // Modify f.txt differently on topic
    run_bit_command(dir.path(), &["checkout", "topic"])
        .assert()
        .success();
    write_file(FileSpec::new(
        dir.path().join("f.txt"),
        "topic change\n".into(),
    ));
    run_bit_command(dir.path(), &["add", "."])
        .assert()
        .success();
    bit_commit(dir.path(), "Commit C - topic")
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
        stdout.contains("both modified"),
        "Expected 'both modified' label in:\n{}",
        stdout
    );
    assert!(
        stdout.contains("f.txt"),
        "Expected 'f.txt' in unmerged section:\n{}",
        stdout
    );

    Ok(())
}
