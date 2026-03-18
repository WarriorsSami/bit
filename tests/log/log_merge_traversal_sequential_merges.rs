/// Test Case 5: Multiple branches merged sequentially
///
/// Tests the real-world scenario of merging multiple feature branches
/// into main over time. The first merge (feature-1 into master) is a
/// fast-forward because master hasn't diverged from feature-1's base yet,
/// so no merge commit is created for it — master simply advances to D.
/// The subsequent two merges (feature-2 and feature-3) are true merges
/// because master has since diverged.
///
/// Timeline:
/// - A: T0 (base on master)
/// - B: T1, D: T3 (feature-1, branched from A)
/// - C: T2 (feature-2, branched from A)
/// - F: T4, G: T6 (feature-3, branched from A)
/// - T5: merge feature-1 → master is a FAST-FORWARD (master moves to D, no commit created)
/// - M2: T7 (true merge of feature-2; master/D and C have diverged)
/// - M3: T8 (true merge of feature-3)
///
/// Expected: 8 unique commits — M3, M2, G, F, D, C, B, A
/// (no separate M1 commit because the first merge was fast-forwarded)
use crate::common::command::{
    bit_commit_with_timestamp, bit_merge_with_timestamp, repository_dir, run_bit_command,
    run_git_command,
};
use crate::common::file::{FileSpec, write_file};
use assert_fs::TempDir;
use rstest::rstest;

#[rstest]
fn log_merge_traversal_sequential_merges(
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

    // Create three feature branches
    run_bit_command(dir, &["branch", "create", "feature-1"])
        .assert()
        .success();
    run_bit_command(dir, &["branch", "create", "feature-2"])
        .assert()
        .success();
    run_bit_command(dir, &["branch", "create", "feature-3"])
        .assert()
        .success();

    // B: Feature-1 first commit (T1)
    run_bit_command(dir, &["checkout", "feature-1"])
        .assert()
        .success();
    write_file(FileSpec::new(
        dir.join("f1_file1.txt"),
        "f1 1\n".to_string(),
    ));
    run_bit_command(dir, &["add", "."]).assert().success();
    bit_commit_with_timestamp(dir, "B", "2024-01-01 11:00:00 +0000")
        .assert()
        .success();

    // C: Feature-2 commit (T2)
    run_bit_command(dir, &["checkout", "feature-2"])
        .assert()
        .success();
    write_file(FileSpec::new(
        dir.join("f2_file1.txt"),
        "f2 1\n".to_string(),
    ));
    run_bit_command(dir, &["add", "."]).assert().success();
    bit_commit_with_timestamp(dir, "C", "2024-01-01 12:00:00 +0000")
        .assert()
        .success();

    // D: Feature-1 second commit (T3)
    run_bit_command(dir, &["checkout", "feature-1"])
        .assert()
        .success();
    write_file(FileSpec::new(
        dir.join("f1_file2.txt"),
        "f1 2\n".to_string(),
    ));
    run_bit_command(dir, &["add", "."]).assert().success();
    bit_commit_with_timestamp(dir, "D", "2024-01-01 13:00:00 +0000")
        .assert()
        .success();

    // F: Feature-3 first commit (T4)
    run_bit_command(dir, &["checkout", "feature-3"])
        .assert()
        .success();
    write_file(FileSpec::new(
        dir.join("f3_file1.txt"),
        "f3 1\n".to_string(),
    ));
    run_bit_command(dir, &["add", "."]).assert().success();
    bit_commit_with_timestamp(dir, "F", "2024-01-01 14:00:00 +0000")
        .assert()
        .success();

    // M1: Merge feature-1 into master (T5)
    run_bit_command(dir, &["checkout", "master"])
        .assert()
        .success();
    bit_merge_with_timestamp(dir, "feature-1", "M1", "2024-01-01 15:00:00 +0000")
        .assert()
        .success();

    // G: Feature-3 second commit (T6)
    run_bit_command(dir, &["checkout", "feature-3"])
        .assert()
        .success();
    write_file(FileSpec::new(
        dir.join("f3_file2.txt"),
        "f3 2\n".to_string(),
    ));
    run_bit_command(dir, &["add", "."]).assert().success();
    bit_commit_with_timestamp(dir, "G", "2024-01-01 16:00:00 +0000")
        .assert()
        .success();

    // M2: Merge feature-2 into master (T7)
    run_bit_command(dir, &["checkout", "master"])
        .assert()
        .success();
    bit_merge_with_timestamp(dir, "feature-2", "M2", "2024-01-01 17:00:00 +0000")
        .assert()
        .success();

    // M3: Merge feature-3 into master (T8)
    bit_merge_with_timestamp(dir, "feature-3", "M3", "2024-01-01 18:00:00 +0000")
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

    // Verify count (8 commits: M3, M2, G, F, D, C, B, A).
    // Note: the merge of feature-1 was a fast-forward (master was at A, a direct
    // ancestor of D), so no merge commit "M1" was created — master just moved to D.
    assert_eq!(
        bit_commits.len(),
        8,
        "Expected 8 commits total, found {}",
        bit_commits.len()
    );

    // Verify order matches git
    assert_eq!(
        bit_commits, git_commits,
        "Bit log output should match git log for sequential merges.\nGit: {:?}\nBit: {:?}",
        git_commits, bit_commits
    );

    // Verify all commits present (M1 is absent: it was a fast-forward)
    for commit in &["M3", "M2", "G", "F", "D", "C", "B", "A"] {
        assert!(
            bit_commits.contains(commit),
            "Commit {} must appear in sequential merge log",
            commit
        );
    }

    // Verify no duplicates
    let mut seen = std::collections::HashSet::new();
    for commit in &bit_commits {
        assert!(
            seen.insert(commit),
            "Commit {} appears multiple times in log output",
            commit
        );
    }

    // Verify chronological ordering (newest first)
    // The exact interleaving depends on timestamps, but M3 should be first
    assert_eq!(bit_commits[0], "M3", "Most recent merge should be first");

    Ok(())
}
