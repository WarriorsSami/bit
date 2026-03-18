use crate::common::command::{
    bit_commit, bit_merge, get_branch_commit_sha, repository_dir, run_bit_command,
};
use crate::common::file::{FileSpec, write_file};
use assert_fs::TempDir;
use rstest::rstest;
use std::fs;

/// Test that a conflicting merge writes MERGE_HEAD and MERGE_MSG into .git/
///
/// Setup is the same two-branch content conflict as merge_content_conflict:
///   A: file.txt = "base\nshared\nmore\n"
///   B (master):  "shared" → "ours change"
///   C (feature): "shared" → "theirs change"
///
/// After a failed merge:
/// - .git/MERGE_HEAD must contain feature's commit OID
/// - .git/MERGE_MSG must contain the merge message string
#[rstest]
fn merge_state_persistence(repository_dir: TempDir) -> Result<(), Box<dyn std::error::Error>> {
    let dir = repository_dir;

    run_bit_command(dir.path(), &["init"]).assert().success();

    // Commit A: base content
    write_file(FileSpec::new(
        dir.path().join("file.txt"),
        "base\nshared\nmore\n".into(),
    ));
    run_bit_command(dir.path(), &["add", "."])
        .assert()
        .success();
    bit_commit(dir.path(), "Commit A").assert().success();

    // Create feature branch at A
    run_bit_command(dir.path(), &["branch", "create", "feature"])
        .assert()
        .success();

    // Commit B on master
    write_file(FileSpec::new(
        dir.path().join("file.txt"),
        "base\nours change\nmore\n".into(),
    ));
    run_bit_command(dir.path(), &["add", "."])
        .assert()
        .success();
    bit_commit(dir.path(), "Commit B - ours").assert().success();

    // Commit C on feature
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
    bit_commit(dir.path(), "Commit C - theirs")
        .assert()
        .success();

    run_bit_command(dir.path(), &["checkout", "master"])
        .assert()
        .success();

    let feature_oid = get_branch_commit_sha(dir.path(), "feature")?;
    let merge_message = "Merge feature for state test";

    // Merge must fail with a conflict
    bit_merge(dir.path(), "feature", merge_message)
        .assert()
        .failure();

    // MERGE_HEAD must contain feature's commit OID
    let merge_head_path = dir.path().join(".git").join("MERGE_HEAD");
    assert!(
        merge_head_path.exists(),
        ".git/MERGE_HEAD should be written on conflict"
    );
    let merge_head = fs::read_to_string(&merge_head_path)?;
    assert_eq!(
        merge_head.trim(),
        feature_oid.trim(),
        "MERGE_HEAD should contain feature's OID"
    );

    // MERGE_MSG must contain the merge message
    let merge_msg_path = dir.path().join(".git").join("MERGE_MSG");
    assert!(
        merge_msg_path.exists(),
        ".git/MERGE_MSG should be written on conflict"
    );
    let merge_msg = fs::read_to_string(&merge_msg_path)?;
    assert!(
        merge_msg.contains(merge_message),
        "MERGE_MSG should contain the merge message, got:\n{}",
        merge_msg
    );

    Ok(())
}
