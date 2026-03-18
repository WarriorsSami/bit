use crate::common::command::{
    bit_commit, bit_merge, get_head_commit_sha, repository_dir, run_bit_command,
};
use crate::common::file::{FileSpec, write_file};
use assert_fs::TempDir;
use rstest::rstest;

/// Test that merging an ancestor branch reports "Already up to date"
///
/// History:
///   A → B → C   (master)
///   ^
///   old
///
/// Merging `old` (A) into master (C): BCA == old, so nothing to do.
#[rstest]
fn merge_reports_already_up_to_date(
    repository_dir: TempDir,
) -> Result<(), Box<dyn std::error::Error>> {
    let dir = repository_dir;

    run_bit_command(dir.path(), &["init"]).assert().success();

    write_file(FileSpec::new(dir.path().join("file.txt"), "base\n".into()));
    run_bit_command(dir.path(), &["add", "."])
        .assert()
        .success();
    bit_commit(dir.path(), "Commit A").assert().success();

    // Create `old` branch pointing at A
    run_bit_command(dir.path(), &["branch", "create", "old"])
        .assert()
        .success();

    write_file(FileSpec::new(
        dir.path().join("file.txt"),
        "base\nB\n".into(),
    ));
    run_bit_command(dir.path(), &["add", "."])
        .assert()
        .success();
    bit_commit(dir.path(), "Commit B").assert().success();

    write_file(FileSpec::new(
        dir.path().join("file.txt"),
        "base\nB\nC\n".into(),
    ));
    run_bit_command(dir.path(), &["add", "."])
        .assert()
        .success();
    bit_commit(dir.path(), "Commit C").assert().success();

    let head_before = get_head_commit_sha(dir.path())?;

    let output = bit_merge(dir.path(), "old", "Null merge")
        .assert()
        .success();
    let stdout = String::from_utf8(output.get_output().stdout.clone())?;

    assert!(
        stdout.contains("Already up to date"),
        "Expected 'Already up to date' message, got:\n{}",
        stdout
    );
    assert_eq!(
        get_head_commit_sha(dir.path())?,
        head_before,
        "HEAD should not advance for a null merge"
    );

    Ok(())
}
