use crate::common::command::{
    bit_commit, bit_merge, get_branch_commit_sha, repository_dir, run_bit_command, run_git_command,
};
use crate::common::file::{FileSpec, write_file};
use assert_fs::TempDir;
use rstest::rstest;
use std::fs;

/// Test that merging a descendant branch fast-forwards the pointer
///
/// History:
///   A   (master, feature created here)
///   |
///   B
///   |
///   C   (feature)
///
/// Merging `feature` into master (A) should fast-forward master to C —
/// no merge commit is created.
#[rstest]
fn merge_fast_forward(repository_dir: TempDir) -> Result<(), Box<dyn std::error::Error>> {
    let dir = repository_dir;

    run_bit_command(dir.path(), &["init"]).assert().success();

    // Commit A on master
    write_file(FileSpec::new(
        dir.path().join("anchor.txt"),
        "anchor\n".into(),
    ));
    run_bit_command(dir.path(), &["add", "."])
        .assert()
        .success();
    bit_commit(dir.path(), "Commit A").assert().success();

    // Create feature branch at A, then add commits B and C on it
    run_bit_command(dir.path(), &["branch", "create", "feature"])
        .assert()
        .success();
    run_bit_command(dir.path(), &["checkout", "feature"])
        .assert()
        .success();

    write_file(FileSpec::new(
        dir.path().join("feature_b.txt"),
        "feature b\n".into(),
    ));
    run_bit_command(dir.path(), &["add", "."])
        .assert()
        .success();
    bit_commit(dir.path(), "Commit B").assert().success();

    write_file(FileSpec::new(
        dir.path().join("feature_c.txt"),
        "feature c\n".into(),
    ));
    run_bit_command(dir.path(), &["add", "."])
        .assert()
        .success();
    bit_commit(dir.path(), "Commit C").assert().success();

    // Switch back to master and merge feature
    run_bit_command(dir.path(), &["checkout", "master"])
        .assert()
        .success();

    bit_merge(dir.path(), "feature", "Fast-forward merge")
        .assert()
        .success();

    // master ref must point to the same commit as feature
    let master_sha = get_branch_commit_sha(dir.path(), "master")?;
    let feature_sha = get_branch_commit_sha(dir.path(), "feature")?;
    assert_eq!(
        master_sha, feature_sha,
        "master should be fast-forwarded to feature's tip"
    );

    // The resulting commit must have exactly one parent (not a merge commit)
    let cat_output = run_git_command(dir.path(), &["cat-file", "commit", &master_sha])
        .assert()
        .success();
    let cat_stdout = String::from_utf8(cat_output.get_output().stdout.clone())?;
    let parent_count = cat_stdout
        .lines()
        .filter(|l| l.starts_with("parent "))
        .count();
    assert_eq!(
        parent_count, 1,
        "Fast-forward should produce a commit with exactly 1 parent, got {}",
        parent_count
    );

    // Workspace should contain feature files
    assert!(
        dir.path().join("feature_b.txt").exists(),
        "feature_b.txt should be present after fast-forward"
    );
    assert!(
        dir.path().join("feature_c.txt").exists(),
        "feature_c.txt should be present after fast-forward"
    );
    assert_eq!(
        fs::read_to_string(dir.path().join("feature_c.txt"))?,
        "feature c\n"
    );

    Ok(())
}
