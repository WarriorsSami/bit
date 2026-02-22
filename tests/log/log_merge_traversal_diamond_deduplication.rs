/// Test Case 6: Diamond merge (de-duplication test)
///
/// Tests that commits reachable via multiple paths appear only once.
/// This is a critical correctness test for the visited-tracking mechanism.
///
/// History:
/// ```
///       A
///      / \
///     B   C
///      \ /
///       D (merge)
/// ```
///
/// Timeline:
/// - A: T0 (base)
/// - B: T1 (left path)
/// - C: T2 (right path)
/// - D: T3 (merge both)
///
/// Key property: A is reachable from D via both B and C paths.
/// A must appear exactly ONCE in the output, not twice.
///
/// Expected: D, C, B, A (4 commits, no duplicates)
use crate::common::command::{
    bit_commit_with_timestamp, bit_merge_with_timestamp, repository_dir, run_bit_command,
    run_git_command,
};
use crate::common::file::{FileSpec, write_file};
use assert_fs::TempDir;
use rstest::rstest;

#[rstest]
fn log_merge_traversal_diamond_deduplication(
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

    // Create branch for right path
    run_bit_command(dir, &["branch", "create", "right"])
        .assert()
        .success();

    // B: Left path (T1)
    write_file(FileSpec::new(dir.join("left.txt"), "left\n".to_string()));
    run_bit_command(dir, &["add", "."]).assert().success();
    bit_commit_with_timestamp(dir, "B", "2024-01-01 11:00:00 +0000")
        .assert()
        .success();

    // C: Right path (T2)
    run_bit_command(dir, &["checkout", "right"])
        .assert()
        .success();
    write_file(FileSpec::new(dir.join("right.txt"), "right\n".to_string()));
    run_bit_command(dir, &["add", "."]).assert().success();
    bit_commit_with_timestamp(dir, "C", "2024-01-01 12:00:00 +0000")
        .assert()
        .success();

    // D: Merge both paths (T3)
    run_bit_command(dir, &["checkout", "master"])
        .assert()
        .success();
    bit_merge_with_timestamp(dir, "right", "D", "2024-01-01 13:00:00 +0000")
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

    // Verify count (exactly 4 commits, no duplicates)
    assert_eq!(
        bit_commits.len(),
        4,
        "Expected exactly 4 commits in diamond merge, found {}",
        bit_commits.len()
    );

    // Verify order matches git
    assert_eq!(
        bit_commits, git_commits,
        "Bit log output should match git log for diamond merge.\nGit: {:?}\nBit: {:?}",
        git_commits, bit_commits
    );

    // CRITICAL: Verify no duplicates
    let mut seen = std::collections::HashSet::new();
    for commit in &bit_commits {
        assert!(
            seen.insert(commit),
            "Commit {} appears multiple times - de-duplication failed!",
            commit
        );
    }

    // Verify all commits present exactly once
    for commit in &["D", "C", "B", "A"] {
        let count = bit_commits.iter().filter(|&&c| c == *commit).count();
        assert_eq!(
            count, 1,
            "Commit {} should appear exactly once, found {} times",
            commit, count
        );
    }

    // Verify expected order
    assert_eq!(bit_commits, vec!["D", "C", "B", "A"]);

    Ok(())
}
