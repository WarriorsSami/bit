/// Test Case 2: Simple merge (two-parent merge commit)
///
/// This is the core test for merge commit traversal.
/// Tests that BOTH parents of a merge commit are traversed.
///
/// History:
/// ```
///       A
///      / \
///     B   C
///     |   |
///     D   E
///      \ /
///       M (merge)
/// ```
///
/// Timeline:
/// - A: T0 (base)
/// - B: T1 (main branch)
/// - C: T2 (feature branch)
/// - D: T3 (main)
/// - E: T4 (feature)
/// - M: T5 (merge E into D)
///
/// Expected order: M(T5), E(T4), D(T3), C(T2), B(T1), A(T0)
///
/// **Critical invariant**: Both E and D must appear in output.
/// Without proper merge traversal, E and C would be missing.
use crate::common::command::{
    bit_commit_with_timestamp, bit_merge_with_timestamp, repository_dir, run_bit_command,
    run_git_command,
};
use crate::common::file::{FileSpec, write_file};
use assert_fs::TempDir;
use rstest::rstest;

#[rstest]
fn log_merge_traversal_simple_merge(
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

    // B: Main branch commit (T1)
    write_file(FileSpec::new(dir.join("main1.txt"), "main 1\n".to_string()));
    run_bit_command(dir, &["add", "."]).assert().success();
    bit_commit_with_timestamp(dir, "B", "2024-01-01 11:00:00 +0000")
        .assert()
        .success();

    // Create feature branch from A
    run_bit_command(dir, &["branch", "create", "feature"])
        .assert()
        .success();

    // D: Continue on main (T3)
    write_file(FileSpec::new(dir.join("main2.txt"), "main 2\n".to_string()));
    run_bit_command(dir, &["add", "."]).assert().success();
    bit_commit_with_timestamp(dir, "D", "2024-01-01 13:00:00 +0000")
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

    // E: Feature branch commit (T4)
    write_file(FileSpec::new(dir.join("feat2.txt"), "feat 2\n".to_string()));
    run_bit_command(dir, &["add", "."]).assert().success();
    bit_commit_with_timestamp(dir, "E", "2024-01-01 14:00:00 +0000")
        .assert()
        .success();

    // Switch back to main
    run_bit_command(dir, &["checkout", "master"])
        .assert()
        .success();

    // M: Merge feature into main (T5)
    bit_merge_with_timestamp(dir, "feature", "M", "2024-01-01 15:00:00 +0000")
        .assert()
        .success();

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

    // Verify count (should be 6: M, E, D, C, B, A)
    assert_eq!(
        bit_commits.len(),
        6,
        "Expected 6 commits (including merge), found {}",
        bit_commits.len()
    );

    // Verify order matches git (timestamp order)
    assert_eq!(
        bit_commits, git_commits,
        "Bit log output should match git log for simple merge.\nGit: {:?}\nBit: {:?}",
        git_commits, bit_commits
    );

    // CRITICAL: Verify both parents are traversed
    assert!(
        bit_commits.contains(&"E"),
        "Feature branch commit E must appear in log (second parent traversal)"
    );
    assert!(
        bit_commits.contains(&"C"),
        "Feature branch commit C must appear in log (second parent traversal)"
    );

    // Verify expected chronological order
    assert_eq!(bit_commits, vec!["M", "E", "D", "C", "B", "A"]);

    Ok(())
}
