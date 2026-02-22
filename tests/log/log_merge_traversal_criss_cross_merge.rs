/// Test Case 3: Criss-cross merge (multiple merge bases)
///
/// This tests complex merge history where branches merge back and forth,
/// creating multiple common ancestors (criss-cross pattern).
///
/// History:
/// ```
///       A
///      / \
///     B   C
///     |\ /|
///     | X |
///     |/ \|
///     D   E
///      \ /
///       M (merge)
/// ```
///
/// Timeline:
/// - A: T0 (base)
/// - B: T1 (main)
/// - C: T2 (feature)
/// - D: T3 (main merges C)
/// - E: T4 (feature merges B)
/// - M: T5 (main merges feature)
///
/// This creates a "criss-cross" where:
/// - D has parents B and C
/// - E has parents C and B
///
/// Expected: All commits (M, E, D, C, B, A) must appear
/// Challenge: Shared ancestors should appear only once
use crate::common::command::{
    bit_commit_with_timestamp, bit_merge_with_timestamp, repository_dir, run_bit_command,
    run_git_command,
};
use crate::common::file::{FileSpec, write_file};
use assert_fs::TempDir;
use rstest::rstest;

#[rstest]
fn log_merge_traversal_criss_cross_merge(
    repository_dir: TempDir,
) -> Result<(), Box<dyn std::error::Error>> {
    let dir = repository_dir.path();

    // Initialize repository
    run_bit_command(dir, &["init"]).assert().success();

    // A: Base commit (T0)
    write_file(FileSpec::new(dir.join("base.txt"), "base\n".to_string()));
    run_bit_command(dir, &["add", "."]).assert().success();
    bit_commit_with_timestamp(dir, "A", "2024-01-01 10:00:00 +0000")
        .assert()
        .success();

    // Create feature branch
    run_bit_command(dir, &["branch", "create", "feature"])
        .assert()
        .success();

    // B: Main branch commit (T1)
    write_file(FileSpec::new(dir.join("main1.txt"), "main 1\n".to_string()));
    run_bit_command(dir, &["add", "."]).assert().success();
    bit_commit_with_timestamp(dir, "B", "2024-01-01 11:00:00 +0000")
        .assert()
        .success();

    // Switch to feature branch
    run_bit_command(dir, &["checkout", "feature"])
        .assert()
        .success();

    // C: Feature branch commit (T2)
    write_file(FileSpec::new(dir.join("feat1.txt"), "feat 1\n".to_string()));
    run_bit_command(dir, &["add", "."]).assert().success();
    bit_commit_with_timestamp(dir, "C", "2024-01-01 12:00:00 +0000")
        .assert()
        .success();

    // D: Main merges feature (T3) - first cross
    run_bit_command(dir, &["checkout", "master"])
        .assert()
        .success();
    bit_merge_with_timestamp(dir, "feature", "D", "2024-01-01 13:00:00 +0000")
        .assert()
        .success();

    // E: Feature merges main (T4) - second cross
    run_bit_command(dir, &["checkout", "feature"])
        .assert()
        .success();
    bit_merge_with_timestamp(dir, "master", "E", "2024-01-01 14:00:00 +0000")
        .assert()
        .success();

    // M: Final merge (T5)
    run_bit_command(dir, &["checkout", "master"])
        .assert()
        .success();
    bit_merge_with_timestamp(dir, "feature", "M", "2024-01-01 15:00:00 +0000")
        .assert()
        .success();

    // Run git log for comparison
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
        6,
        "Expected 6 commits in criss-cross merge, found {}",
        bit_commits.len()
    );

    // Verify order matches git
    assert_eq!(
        bit_commits, git_commits,
        "Bit log output should match git log for criss-cross merge.\nGit: {:?}\nBit: {:?}",
        git_commits, bit_commits
    );

    // Verify all commits present
    for commit in &["M", "E", "D", "C", "B", "A"] {
        assert!(
            bit_commits.contains(commit),
            "Commit {} must appear in criss-cross merge log",
            commit
        );
    }

    // Verify no duplicates (each commit appears exactly once)
    let mut seen = std::collections::HashSet::new();
    for commit in &bit_commits {
        assert!(
            seen.insert(commit),
            "Commit {} appears multiple times in log output",
            commit
        );
    }

    Ok(())
}
