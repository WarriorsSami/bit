use crate::common::command::{bit_commit, bit_merge, repository_dir, run_bit_command};
use crate::common::file::{FileSpec, write_file};
use assert_fs::TempDir;
use rstest::rstest;

/// Verifies that both `bit commit` and `bit merge --continue` refuse to finalize
/// when unresolved conflict stages are still present in the index.
#[rstest]
fn merge_resume_dirty_guard(repository_dir: TempDir) -> Result<(), Box<dyn std::error::Error>> {
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

    bit_merge(dir.path(), "feature", "Merge feature")
        .assert()
        .failure();

    // Do NOT resolve — leave stages 1-3 in place

    // `bit commit` must fail with an error mentioning unmerged files
    let mut commit_cmd = run_bit_command(dir.path(), &["commit"]);
    commit_cmd.envs(vec![
        ("GIT_AUTHOR_NAME", "fake_user"),
        ("GIT_AUTHOR_EMAIL", "fake_email@email.com"),
        ("GIT_AUTHOR_DATE", "2023-01-01 12:00:00 +0000"),
    ]);
    let commit_out = commit_cmd.assert().failure().get_output().stderr.clone();
    let commit_err = String::from_utf8(commit_out)?;
    assert!(
        commit_err.to_lowercase().contains("unmerged")
            || commit_err.to_lowercase().contains("conflict"),
        "commit should mention unmerged/conflict, got:\n{}",
        commit_err
    );

    // `bit merge --continue` must also fail
    let mut continue_cmd = run_bit_command(dir.path(), &["merge", "--continue"]);
    continue_cmd.envs(vec![
        ("GIT_AUTHOR_NAME", "fake_user"),
        ("GIT_AUTHOR_EMAIL", "fake_email@email.com"),
        ("GIT_AUTHOR_DATE", "2023-01-01 12:00:00 +0000"),
    ]);
    let continue_out = continue_cmd.assert().failure().get_output().stderr.clone();
    let continue_err = String::from_utf8(continue_out)?;
    assert!(
        continue_err.to_lowercase().contains("unmerged")
            || continue_err.to_lowercase().contains("conflict"),
        "merge --continue should mention unmerged/conflict, got:\n{}",
        continue_err
    );

    Ok(())
}
