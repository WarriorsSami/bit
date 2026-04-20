use crate::common::command::{bit_commit, bit_merge, repository_dir, run_bit_command};
use crate::common::file::{FileSpec, write_file};
use assert_fs::TempDir;
use rstest::rstest;

/// `bit diff` without a stage flag shows the unmerged notice but no patch (no --- / +++ lines).
#[rstest]
fn no_patch_without_stage_flag(repository_dir: TempDir) -> Result<(), Box<dyn std::error::Error>> {
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

    write_file(FileSpec::new(dir.path().join("f.txt"), "ours\n".into()));
    run_bit_command(dir.path(), &["add", "."])
        .assert()
        .success();
    bit_commit(dir.path(), "B").assert().success();

    run_bit_command(dir.path(), &["checkout", "feature"])
        .assert()
        .success();
    write_file(FileSpec::new(dir.path().join("f.txt"), "theirs\n".into()));
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

    let out = run_bit_command(dir.path(), &["diff"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let text = String::from_utf8(out)?;

    assert!(
        text.contains("* Unmerged path f.txt"),
        "Expected unmerged notice, got:\n{}",
        text
    );
    assert!(
        !text.contains("---") && !text.contains("+++"),
        "Expected no patch lines without stage flag, got:\n{}",
        text
    );

    Ok(())
}
