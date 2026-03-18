use crate::common::command::{
    bit_commit, bit_merge, repository_dir, run_bit_command, run_git_command,
};
use crate::common::file::{FileSpec, write_file};
use assert_fs::TempDir;
use rstest::rstest;
use std::fs;

/// Test that concurrent modifications to the same lines produce conflict markers
///
/// History:
///   A: file.txt = "base\nshared\nmore\n"
///   B (master):  "shared" → "ours change"
///   C (feature): "shared" → "theirs change"
///
/// Expected: non-zero exit, conflict markers in file.txt, 3 index stages
#[rstest]
fn merge_content_conflict(repository_dir: TempDir) -> Result<(), Box<dyn std::error::Error>> {
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

    // Commit B on master: modify the shared line
    write_file(FileSpec::new(
        dir.path().join("file.txt"),
        "base\nours change\nmore\n".into(),
    ));
    run_bit_command(dir.path(), &["add", "."])
        .assert()
        .success();
    bit_commit(dir.path(), "Commit B - ours").assert().success();

    // Commit C on feature: modify the same shared line differently
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

    // Merge must fail with a conflict
    bit_merge(dir.path(), "feature", "conflict merge")
        .assert()
        .failure();

    // Conflict markers must be present in the working file
    let content = fs::read_to_string(dir.path().join("file.txt"))?;
    assert!(
        content.contains("<<<<<<<"),
        "Expected conflict marker '<<<<<<<' in file.txt"
    );
    assert!(
        content.contains("======="),
        "Expected conflict separator '=======' in file.txt"
    );
    assert!(
        content.contains(">>>>>>>"),
        "Expected conflict marker '>>>>>>>' in file.txt"
    );
    assert!(
        content.contains("ours change"),
        "Expected 'ours change' in conflict output"
    );
    assert!(
        content.contains("theirs change"),
        "Expected 'theirs change' in conflict output"
    );

    // Index must have 3 stages for file.txt (base=1, ours=2, theirs=3)
    // Use real `git` because bit has no ls-files command; bit writes a git-compatible index.
    let stage_output = run_git_command(dir.path(), &["ls-files", "--stage"])
        .assert()
        .success();
    let stage_out = String::from_utf8(stage_output.get_output().stdout.clone())?;
    let entries: Vec<&str> = stage_out
        .lines()
        .filter(|l| l.contains("file.txt"))
        .collect();

    assert_eq!(
        entries.len(),
        3,
        "Expected 3 index stages for file.txt (base/ours/theirs), got:\n{}",
        stage_out
    );
    // git ls-files --stage format: "<mode> <hash> <stage>\t<path>"
    // Stage is the third space-separated field before the tab.
    let stage_of = |line: &&str| -> Option<u8> {
        line.split('\t')
            .next()
            .and_then(|pre| pre.split_whitespace().nth(2))
            .and_then(|s| s.parse().ok())
    };
    assert!(
        entries.iter().any(|e| stage_of(e) == Some(1)),
        "Missing stage 1 (base) for file.txt"
    );
    assert!(
        entries.iter().any(|e| stage_of(e) == Some(2)),
        "Missing stage 2 (ours) for file.txt"
    );
    assert!(
        entries.iter().any(|e| stage_of(e) == Some(3)),
        "Missing stage 3 (theirs) for file.txt"
    );

    Ok(())
}
