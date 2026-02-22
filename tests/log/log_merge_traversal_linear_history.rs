/// Test Case 1: Linear history (baseline - no merge commits)
///
/// This test establishes the baseline behavior for linear history.
/// All subsequent merge tests will be compared against this pattern.
///
/// History:
/// ```
/// A <- B <- C <- D <- E
/// ```
///
/// Expected: E, D, C, B, A (chronological order)
use crate::common::command::{
    bit_commit_with_timestamp, repository_dir, run_bit_command, run_git_command,
};
use crate::common::file::{FileSpec, write_file};
use assert_fs::TempDir;
use rstest::rstest;

#[rstest]
fn log_merge_traversal_linear_history(
    repository_dir: TempDir,
) -> Result<(), Box<dyn std::error::Error>> {
    let dir = repository_dir.path();

    // Initialize repository
    run_bit_command(dir, &["init"]).assert().success();

    // Create linear history with timestamps
    let commits = [
        ("A", "2024-01-01 10:00:00 +0000", "a.txt", "A content"),
        ("B", "2024-01-01 11:00:00 +0000", "b.txt", "B content"),
        ("C", "2024-01-01 12:00:00 +0000", "c.txt", "C content"),
        ("D", "2024-01-01 13:00:00 +0000", "d.txt", "D content"),
        ("E", "2024-01-01 14:00:00 +0000", "e.txt", "E content"),
    ];

    for (msg, timestamp, file, content) in commits {
        write_file(FileSpec::new(dir.join(file), content.to_string()));
        run_bit_command(dir, &["add", "."]).assert().success();
        bit_commit_with_timestamp(dir, msg, timestamp)
            .assert()
            .success();
    }

    // Run git log for comparison (golden test)
    let git_output = run_git_command(dir, &["log", "--format=%s"]).output()?;
    let git_stdout = String::from_utf8(git_output.stdout)?;
    let git_commits: Vec<&str> = git_stdout.lines().collect();

    // Run bit log
    let bit_output = run_bit_command(dir, &["log", "--format=oneline", "--decorate=none"])
        .assert()
        .success();
    let bit_stdout = String::from_utf8(bit_output.get_output().stdout.clone())?;
    let bit_commits: Vec<&str> = bit_stdout
        .lines()
        .filter_map(|line| line.split_whitespace().nth(1))
        .collect();

    // Verify count
    assert_eq!(
        bit_commits.len(),
        5,
        "Expected 5 commits in linear history, found {}",
        bit_commits.len()
    );

    // Verify order matches git
    assert_eq!(
        bit_commits, git_commits,
        "Bit log output should match git log for linear history.\nGit: {:?}\nBit: {:?}",
        git_commits, bit_commits
    );

    // Verify expected order
    assert_eq!(bit_commits, vec!["E", "D", "C", "B", "A"]);

    Ok(())
}
