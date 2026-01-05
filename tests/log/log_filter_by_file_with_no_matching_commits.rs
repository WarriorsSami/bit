use crate::common::command::{bit_commit, repository_dir, run_bit_command};
use crate::common::file::{FileSpec, write_file};
use assert_fs::TempDir;
use rstest::rstest;

#[rstest]
fn log_filter_by_file_with_no_matching_commits(
    repository_dir: TempDir,
) -> Result<(), Box<dyn std::error::Error>> {
    // This test verifies that filtering by a file that was never committed shows no results

    run_bit_command(repository_dir.path(), &["init"])
        .assert()
        .success();

    // Commit 1: Add file1.txt
    let file1 = FileSpec::new(
        repository_dir.path().join("file1.txt"),
        "content".to_string(),
    );
    write_file(file1);
    run_bit_command(repository_dir.path(), &["add", "."])
        .assert()
        .success();
    bit_commit(repository_dir.path(), "Add file1.txt")
        .assert()
        .success();

    // Commit 2: Add file2.txt
    let file2 = FileSpec::new(
        repository_dir.path().join("file2.txt"),
        "content".to_string(),
    );
    write_file(file2);
    run_bit_command(repository_dir.path(), &["add", "."])
        .assert()
        .success();
    bit_commit(repository_dir.path(), "Add file2.txt")
        .assert()
        .success();

    // Test: log -- nonexistent.txt (should show no commits)
    let output = run_bit_command(
        repository_dir.path(),
        &["log", "--decorate=none", "--", "nonexistent.txt"],
    )
    .assert()
    .success();

    let stdout = String::from_utf8(output.get_output().stdout.clone())?;

    // Should not include any commits
    assert!(
        !stdout.contains("    Add file1.txt"),
        "Should not include any commits when filtering by nonexistent file"
    );
    assert!(
        !stdout.contains("    Add file2.txt"),
        "Should not include any commits when filtering by nonexistent file"
    );

    // Verify no commits are shown (or just "No commits to show.")
    let commit_count = stdout
        .lines()
        .filter(|line| line.starts_with("commit "))
        .count();
    assert_eq!(
        commit_count, 0,
        "Expected 0 commits when filtering by file that was never committed"
    );

    Ok(())
}
