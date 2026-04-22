use crate::common::command::{
    bit_commit, bit_merge, get_branch_commit_sha, repository_dir, run_bit_command, run_git_command,
};
use crate::common::file::{FileSpec, write_file};
use assert_fs::TempDir;
use rstest::rstest;

/// Verifies that `bit merge --continue` finalizes a conflicted merge,
/// produces a two-parent commit, and removes "Unmerged paths" from status output.
#[rstest]
fn merge_resume_continue_flag(repository_dir: TempDir) -> Result<(), Box<dyn std::error::Error>> {
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

    let feature_oid = get_branch_commit_sha(dir.path(), "feature")?;

    bit_merge(dir.path(), "feature", "Merge feature into master")
        .assert()
        .failure();

    // Resolve conflict
    write_file(FileSpec::new(
        dir.path().join("file.txt"),
        "base\nresolved\nmore\n".into(),
    ));
    run_bit_command(dir.path(), &["add", "file.txt"])
        .assert()
        .success();

    // Resume via --continue (author env vars needed)
    let mut cmd = run_bit_command(dir.path(), &["merge", "--continue"]);
    cmd.envs(vec![
        ("GIT_AUTHOR_NAME", "fake_user"),
        ("GIT_AUTHOR_EMAIL", "fake_email@email.com"),
        ("GIT_AUTHOR_DATE", "2023-01-01 12:00:00 +0000"),
    ]);
    cmd.assert().success();

    // Two-parent commit
    let cat = run_git_command(dir.path(), &["cat-file", "commit", "HEAD"])
        .assert()
        .success();
    let cat_out = String::from_utf8(cat.get_output().stdout.clone())?;
    let parents: Vec<&str> = cat_out
        .lines()
        .filter(|l| l.starts_with("parent "))
        .collect();

    assert_eq!(
        parents.len(),
        2,
        "merge commit must have 2 parents, got:\n{}",
        cat_out
    );
    assert!(
        parents[1].contains(feature_oid.trim()),
        "second parent must be feature OID"
    );

    // Status must not mention unmerged paths
    let status = run_bit_command(dir.path(), &["status"]).assert().success();
    let status_out = String::from_utf8(status.get_output().stdout.clone())?;
    assert!(
        !status_out.contains("Unmerged paths"),
        "status should not show 'Unmerged paths' after successful merge, got:\n{}",
        status_out
    );

    Ok(())
}
