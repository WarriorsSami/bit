use crate::common::command::{bit_commit, bit_merge, repository_dir, run_bit_command};
use crate::common::file::{FileSpec, write_file};
use assert_fs::TempDir;
use rstest::rstest;

/// Running `bit add` on a conflicted file evicts the conflict stages and replaces them with
/// a single stage-0 entry, moving the file from "Unmerged paths" to "Changes to be committed".
///
/// History:
///   A: f.txt = "base\n"
///   B (master):  f.txt = "master change\n"
///   C (topic):   f.txt = "topic change\n"
///
/// After merge conflict, manually resolve and `add` the file:
///   The file must disappear from "Unmerged paths" and appear in "Changes to be committed".
#[rstest]
fn report_conflict_resolved_by_add(
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

    // Produce the conflict
    bit_merge(dir.path(), "topic", "merge topic")
        .assert()
        .failure();

    // Verify conflict is present before resolution
    let before = run_bit_command(dir.path(), &["status"]).assert().success();
    let before_stdout = String::from_utf8(before.get_output().stdout.clone())?;
    assert!(
        before_stdout.contains("Unmerged paths"),
        "Expected conflict before add in:\n{}",
        before_stdout
    );

    // Resolve: write a resolution and stage it
    write_file(FileSpec::new(
        dir.path().join("f.txt"),
        "resolved content\n".into(),
    ));
    run_bit_command(dir.path(), &["add", "f.txt"])
        .assert()
        .success();

    // After add, f.txt must move out of "Unmerged paths" into "Changes to be committed"
    let after = run_bit_command(dir.path(), &["status"]).assert().success();
    let after_stdout = String::from_utf8(after.get_output().stdout.clone())?;

    assert!(
        !after_stdout.contains("Unmerged paths"),
        "Expected no 'Unmerged paths' after resolving conflict, got:\n{}",
        after_stdout
    );
    assert!(
        after_stdout.contains("Changes to be committed"),
        "Expected 'Changes to be committed' after resolving conflict in:\n{}",
        after_stdout
    );
    assert!(
        after_stdout.contains("f.txt"),
        "Expected f.txt in 'Changes to be committed':\n{}",
        after_stdout
    );

    Ok(())
}
