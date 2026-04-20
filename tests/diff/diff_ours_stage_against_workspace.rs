use crate::common::command::{bit_commit, bit_merge, repository_dir, run_bit_command};
use crate::common::file::{FileSpec, write_file};
use assert_fs::TempDir;
use rstest::rstest;

/// `bit diff --ours` compares stage 2 (ours) against the workspace file.
#[rstest]
fn diff_ours_stage_against_workspace(
    repository_dir: TempDir,
) -> Result<(), Box<dyn std::error::Error>> {
    let dir = repository_dir;

    run_bit_command(dir.path(), &["init"]).assert().success();

    write_file(FileSpec::new(dir.path().join("f.txt"), "base\n".into()));
    run_bit_command(dir.path(), &["add", "."])
        .assert()
        .success();
    bit_commit(dir.path(), "A").assert().success();

    run_bit_command(dir.path(), &["branch", "create", "feature"])
        .assert()
        .success();

    write_file(FileSpec::new(dir.path().join("f.txt"), "Ours\n".into()));
    run_bit_command(dir.path(), &["add", "."])
        .assert()
        .success();
    bit_commit(dir.path(), "B").assert().success();

    run_bit_command(dir.path(), &["checkout", "feature"])
        .assert()
        .success();
    write_file(FileSpec::new(dir.path().join("f.txt"), "Theirs\n".into()));
    run_bit_command(dir.path(), &["add", "."])
        .assert()
        .success();
    bit_commit(dir.path(), "C").assert().success();

    run_bit_command(dir.path(), &["checkout", "master"])
        .assert()
        .success();
    bit_merge(dir.path(), "feature", "conflict")
        .assert()
        .failure();

    let out = run_bit_command(dir.path(), &["diff", "--ours"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let text = String::from_utf8(out)?;

    // The patch should be present (stage-2 is the "before" state)
    assert!(
        text.contains("--- a/f.txt") && text.contains("+++ b/f.txt"),
        "Expected a patch for f.txt, got:\n{}",
        text
    );
    // Conflict markers from the workspace appear as added lines
    assert!(
        text.contains("+<<<<<<< HEAD"),
        "Expected workspace conflict marker as added line, got:\n{}",
        text
    );
    assert!(
        !text.contains("* Unmerged path"),
        "Should not print unmerged notice when stage flag given, got:\n{}",
        text
    );

    Ok(())
}
