use crate::common::command::{
    bit_commit, bit_merge, repository_dir, run_bit_command, run_git_command,
};
use crate::common::file::{FileSpec, write_file};
use assert_fs::TempDir;
use rstest::rstest;

/// Verifies that `bit add <file>` evicts conflict stages 1-3 and installs a clean stage-0 entry.
///
/// After a content conflict, git ls-files --stage shows stages 1/2/3 for file.txt.
/// After writing resolved content and running `bit add file.txt`, only stage 0 must remain.
#[rstest]
fn merge_resume_index_clearing(repository_dir: TempDir) -> Result<(), Box<dyn std::error::Error>> {
    let dir = repository_dir;
    run_bit_command(dir.path(), &["init"]).assert().success();

    write_file(FileSpec::new(
        dir.path().join("file.txt"),
        "base\nshared\nmore\n".into(),
    ));
    run_bit_command(dir.path(), &["add", "."])
        .assert()
        .success();
    bit_commit(dir.path(), "A").assert().success();

    run_bit_command(dir.path(), &["branch", "create", "feature"])
        .assert()
        .success();

    write_file(FileSpec::new(
        dir.path().join("file.txt"),
        "base\nours change\nmore\n".into(),
    ));
    run_bit_command(dir.path(), &["add", "."])
        .assert()
        .success();
    bit_commit(dir.path(), "B - ours").assert().success();

    run_bit_command(dir.path(), &["checkout", "feature"])
        .assert()
        .success();
    write_file(FileSpec::new(
        dir.path().join("file.txt"),
        "base\ntheirs change\nmore\n".into(),
    ));
    run_bit_command(dir.path(), &["add", "."])
        .assert()
        .success();
    bit_commit(dir.path(), "C - theirs").assert().success();

    run_bit_command(dir.path(), &["checkout", "master"])
        .assert()
        .success();

    bit_merge(dir.path(), "feature", "merge feature")
        .assert()
        .failure();

    // Confirm stages 1-3 exist before resolution
    let before = run_git_command(dir.path(), &["ls-files", "--stage"])
        .assert()
        .success();
    let before_out = String::from_utf8(before.get_output().stdout.clone())?;
    let conflict_entries: Vec<&str> = before_out
        .lines()
        .filter(|l| l.contains("file.txt"))
        .collect();
    assert_eq!(
        conflict_entries.len(),
        3,
        "expected 3 conflict stages before resolution"
    );

    // Resolve: write clean content and stage it
    write_file(FileSpec::new(
        dir.path().join("file.txt"),
        "base\nresolved\nmore\n".into(),
    ));
    run_bit_command(dir.path(), &["add", "file.txt"])
        .assert()
        .success();

    // After add, only stage 0 should remain for file.txt
    let after = run_git_command(dir.path(), &["ls-files", "--stage"])
        .assert()
        .success();
    let after_out = String::from_utf8(after.get_output().stdout.clone())?;
    let entries: Vec<&str> = after_out
        .lines()
        .filter(|l| l.contains("file.txt"))
        .collect();

    assert_eq!(
        entries.len(),
        1,
        "expected exactly one stage-0 entry for file.txt after add, got:\n{}",
        after_out
    );

    let stage_of = |line: &&str| -> Option<u8> {
        line.split('\t')
            .next()
            .and_then(|pre| pre.split_whitespace().nth(2))
            .and_then(|s| s.parse().ok())
    };
    assert_eq!(
        stage_of(&entries[0]),
        Some(0),
        "entry should be stage 0, got:\n{}",
        entries[0]
    );

    Ok(())
}
