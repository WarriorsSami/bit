use crate::common::command::{bit_commit, bit_merge, repository_dir, run_bit_command};
use crate::common::file::{FileSpec, write_file};
use assert_fs::TempDir;
use rstest::rstest;

/// Test that an add/add conflict emits a "CONFLICT (add/add)" progress line
///
/// History:
///   A: anchor.txt only
///   B (master):  new.txt = "ours\n"
///   C (feature): new.txt = "theirs\n"
///
/// Expected stdout:
///   CONFLICT (add/add): Merge conflict in new.txt
#[rstest]
fn merge_conflict_report_add_add(
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

    // Commit B on master: add new.txt with "ours"
    write_file(FileSpec::new(dir.path().join("new.txt"), "ours\n".into()));
    run_bit_command(dir.path(), &["add", "."])
        .assert()
        .success();
    bit_commit(dir.path(), "Commit B - add new.txt ours")
        .assert()
        .success();

    // Commit C on feature: add new.txt with "theirs"
    run_bit_command(dir.path(), &["checkout", "feature"])
        .assert()
        .success();
    write_file(FileSpec::new(dir.path().join("new.txt"), "theirs\n".into()));
    run_bit_command(dir.path(), &["add", "."])
        .assert()
        .success();
    bit_commit(dir.path(), "Commit C - add new.txt theirs")
        .assert()
        .success();

    run_bit_command(dir.path(), &["checkout", "master"])
        .assert()
        .success();

    // Merge must fail with a conflict
    let output = bit_merge(dir.path(), "feature", "add-add conflict")
        .assert()
        .failure();
    let stdout = String::from_utf8(output.get_output().stdout.clone())?;

    assert!(
        stdout.contains("CONFLICT (add/add): Merge conflict in new.txt"),
        "Expected 'CONFLICT (add/add): Merge conflict in new.txt' in stdout, got:\n{}",
        stdout
    );

    Ok(())
}
