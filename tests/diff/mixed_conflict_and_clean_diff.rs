use crate::common::command::{bit_commit, bit_merge, repository_dir, run_bit_command};
use crate::common::file::{FileSpec, write_file};
use assert_fs::TempDir;
use rstest::rstest;

/// `bit diff` shows unmerged signal for f.txt and a normal patch for clean g.txt.
#[rstest]
fn mixed_conflict_and_clean_diff(
    repository_dir: TempDir,
) -> Result<(), Box<dyn std::error::Error>> {
    let dir = repository_dir;

    run_bit_command(dir.path(), &["init"]).assert().success();

    write_file(FileSpec::new(dir.path().join("f.txt"), "base\n".into()));
    write_file(FileSpec::new(dir.path().join("g.txt"), "original\n".into()));
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

    // Make an unstaged change to the clean g.txt
    write_file(FileSpec::new(dir.path().join("g.txt"), "modified\n".into()));

    let out = run_bit_command(dir.path(), &["diff"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let text = String::from_utf8(out)?;

    assert!(
        text.contains("* Unmerged path f.txt"),
        "Expected unmerged signal for f.txt, got:\n{}",
        text
    );
    assert!(
        text.contains("g.txt"),
        "Expected patch for g.txt, got:\n{}",
        text
    );
    assert!(
        text.contains("-original") || text.contains("+modified"),
        "Expected diff hunk for g.txt, got:\n{}",
        text
    );

    Ok(())
}
