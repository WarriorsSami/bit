use crate::common::command::{
    bit_commit, bit_merge, get_branch_commit_sha, repository_dir, run_bit_command, run_git_command,
};
use crate::common::file::{FileSpec, write_file};
use assert_fs::TempDir;
use rstest::rstest;

/// Verifies that `bit commit` (no -m) finalizes a conflicted merge using MERGE_MSG,
/// produces a two-parent commit, and clears MERGE_HEAD / MERGE_MSG.
#[rstest]
fn merge_resume_commit(repository_dir: TempDir) -> Result<(), Box<dyn std::error::Error>> {
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

    // Commit without -m; should use MERGE_MSG (env vars still needed for author info)
    let mut cmd = run_bit_command(dir.path(), &["commit"]);
    cmd.envs(vec![
        ("GIT_AUTHOR_NAME", "fake_user"),
        ("GIT_AUTHOR_EMAIL", "fake_email@email.com"),
        ("GIT_AUTHOR_DATE", "2023-01-01 12:00:00 +0000"),
    ]);
    cmd.assert().success();

    // The resulting commit must have two parents
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
        "second parent must be feature OID ({}), got:\n{}",
        feature_oid.trim(),
        parents[1]
    );

    // MERGE_HEAD and MERGE_MSG must be gone
    assert!(
        !dir.path().join(".git").join("MERGE_HEAD").exists(),
        "MERGE_HEAD should be deleted after successful merge commit"
    );
    assert!(
        !dir.path().join(".git").join("MERGE_MSG").exists(),
        "MERGE_MSG should be deleted after successful merge commit"
    );

    Ok(())
}
