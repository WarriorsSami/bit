use crate::common::command::{bit_commit, bit_merge, repository_dir, run_bit_command};
use crate::common::file::{FileSpec, write_file};
use assert_fs::TempDir;
use pretty_assertions::assert_eq;
use rstest::rstest;

/// Porcelain status uses "UU" for a both-modified (edit/edit) conflict.
///
/// History:
///   A: f.txt = "base\n"
///   B (master):  f.txt = "master change\n"
///   C (topic):   f.txt = "topic change\n"
///
/// After merging topic into master (fails), `status --porcelain` must output "UU f.txt".
#[rstest]
fn report_conflict_porcelain_both_modified(
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

    bit_merge(dir.path(), "topic", "merge topic")
        .assert()
        .failure();

    let output = run_bit_command(dir.path(), &["status", "--porcelain"])
        .assert()
        .success();
    let stdout = String::from_utf8(output.get_output().stdout.clone())?;

    assert_eq!(stdout, "UU f.txt\n");

    Ok(())
}
