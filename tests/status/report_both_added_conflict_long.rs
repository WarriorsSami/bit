use crate::common::command::{bit_commit, bit_merge, repository_dir, run_bit_command};
use crate::common::file::{FileSpec, write_file};
use assert_fs::TempDir;
use rstest::rstest;

/// Long-format status shows "both added" when both branches independently add the same file.
///
/// History:
///   A: anchor.txt only (g.txt does not exist)
///   B (master):  g.txt = "master content\n"
///   C (topic):   g.txt = "topic content\n"
///
/// After merging topic into master (fails with conflict), status must show
/// "both added:      g.txt" in "Unmerged paths".
#[rstest]
fn report_both_added_conflict_long(
    repository_dir: TempDir,
) -> Result<(), Box<dyn std::error::Error>> {
    let dir = repository_dir;

    run_bit_command(dir.path(), &["init"]).assert().success();

    // Commit A: anchor only — g.txt does not yet exist
    write_file(FileSpec::new(
        dir.path().join("anchor.txt"),
        "anchor\n".into(),
    ));
    run_bit_command(dir.path(), &["add", "."])
        .assert()
        .success();
    bit_commit(dir.path(), "Commit A").assert().success();

    run_bit_command(dir.path(), &["branch", "create", "topic"])
        .assert()
        .success();

    // Add g.txt on master
    write_file(FileSpec::new(
        dir.path().join("g.txt"),
        "master content\n".into(),
    ));
    run_bit_command(dir.path(), &["add", "."])
        .assert()
        .success();
    bit_commit(dir.path(), "Commit B - add g.txt master")
        .assert()
        .success();

    // Add g.txt on topic with different content
    run_bit_command(dir.path(), &["checkout", "topic"])
        .assert()
        .success();
    write_file(FileSpec::new(
        dir.path().join("g.txt"),
        "topic content\n".into(),
    ));
    run_bit_command(dir.path(), &["add", "."])
        .assert()
        .success();
    bit_commit(dir.path(), "Commit C - add g.txt topic")
        .assert()
        .success();

    run_bit_command(dir.path(), &["checkout", "master"])
        .assert()
        .success();

    // Merge fails due to add/add conflict
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
        stdout.contains("both added"),
        "Expected 'both added' label in:\n{}",
        stdout
    );
    assert!(
        stdout.contains("g.txt"),
        "Expected 'g.txt' in unmerged section:\n{}",
        stdout
    );

    Ok(())
}
