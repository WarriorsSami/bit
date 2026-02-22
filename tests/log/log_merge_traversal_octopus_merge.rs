/// Test Case 4: Octopus merge (3+ parents)
///
/// Tests traversal of merge commits with more than 2 parents.
/// Git supports "octopus merges" - merging multiple branches simultaneously.
///
/// History:
/// ```
///         A
///       / | \
///      B  C  D
///       \ | /
///         M (octopus merge of B, C, D)
/// ```
///
/// Timeline:
/// - A: T0 (base)
/// - B: T1 (branch-1)
/// - C: T2 (branch-2)
/// - D: T3 (branch-3)
/// - M: T4 (merge all three)
///
/// Expected: M, D, C, B, A (all commits present)
///
/// **Critical**: ALL three parents (B, C, D) must be traversed.
/// This tests that the implementation doesn't assume exactly 2 parents.
use crate::common::command::{
    bit_commit_with_timestamp, repository_dir, run_bit_command, run_git_command,
};
use crate::common::file::{FileSpec, write_file};
use assert_fs::TempDir;
use rstest::rstest;

#[rstest]
#[ignore = "Octopus merge support may not be implemented yet, but this test will be critical to add when it is."]
fn log_merge_traversal_octopus_merge(
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

    // Create three branches
    run_bit_command(dir, &["branch", "create", "branch-1"])
        .assert()
        .success();
    run_bit_command(dir, &["branch", "create", "branch-2"])
        .assert()
        .success();
    run_bit_command(dir, &["branch", "create", "branch-3"])
        .assert()
        .success();

    // B: Branch-1 commit (T1)
    run_bit_command(dir, &["checkout", "branch-1"])
        .assert()
        .success();
    write_file(FileSpec::new(dir.join("b.txt"), "branch 1\n".to_string()));
    run_bit_command(dir, &["add", "."]).assert().success();
    bit_commit_with_timestamp(dir, "B", "2024-01-01 11:00:00 +0000")
        .assert()
        .success();

    // C: Branch-2 commit (T2)
    run_bit_command(dir, &["checkout", "branch-2"])
        .assert()
        .success();
    write_file(FileSpec::new(dir.join("c.txt"), "branch 2\n".to_string()));
    run_bit_command(dir, &["add", "."]).assert().success();
    bit_commit_with_timestamp(dir, "C", "2024-01-01 12:00:00 +0000")
        .assert()
        .success();

    // D: Branch-3 commit (T3)
    run_bit_command(dir, &["checkout", "branch-3"])
        .assert()
        .success();
    write_file(FileSpec::new(dir.join("d.txt"), "branch 3\n".to_string()));
    run_bit_command(dir, &["add", "."]).assert().success();
    bit_commit_with_timestamp(dir, "D", "2024-01-01 13:00:00 +0000")
        .assert()
        .success();

    // Return to master for octopus merge
    run_bit_command(dir, &["checkout", "master"])
        .assert()
        .success();

    // Perform octopus merge using git (bit may not support octopus merge syntax yet)
    // This creates a commit with 3 parents
    let merge_result = run_git_command(
        dir,
        &[
            "-c",
            "user.name=fake_user",
            "-c",
            "user.email=fake_email@email.com",
            "merge",
            "--no-ff",
            "-m",
            "M",
            "branch-1",
            "branch-2",
            "branch-3",
        ],
    )
    .env("GIT_AUTHOR_DATE", "2024-01-01 14:00:00 +0000")
    .env("GIT_COMMITTER_DATE", "2024-01-01 14:00:00 +0000")
    .output()?;

    assert!(
        merge_result.status.success(),
        "Octopus merge failed: {}",
        String::from_utf8_lossy(&merge_result.stderr)
    );

    // Verify the merge commit has 3 parents
    let cat_output = run_git_command(dir, &["cat-file", "commit", "HEAD"]).output()?;
    let commit_content = String::from_utf8(cat_output.stdout)?;
    let parent_count = commit_content
        .lines()
        .filter(|l| l.starts_with("parent "))
        .count();
    assert!(
        parent_count >= 3,
        "Merge commit should have at least 3 parents, found {}",
        parent_count
    );

    // print the commit content for debugging
    println!("Merge commit content:\n{}", commit_content);

    // Run git log for comparison
    let git_output = run_git_command(dir, &["log", "--format=oneline"]).output()?;
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

    println!("{bit_stdout}");
    println!("{git_stdout}");
    println!("{git_commits:?}");

    // Verify count (5 commits: M, D, C, B, A)
    assert_eq!(
        bit_commits.len(),
        5,
        "Expected 5 commits in octopus merge, found {}",
        bit_commits.len()
    );

    // Verify order matches git
    assert_eq!(
        bit_commits, git_commits,
        "Bit log output should match git log for octopus merge.\nGit: {:?}\nBit: {:?}",
        git_commits, bit_commits
    );

    // CRITICAL: Verify all three branches are traversed
    assert!(
        bit_commits.contains(&"B"),
        "Branch-1 commit B must appear (first parent traversal)"
    );
    assert!(
        bit_commits.contains(&"C"),
        "Branch-2 commit C must appear (second parent traversal)"
    );
    assert!(
        bit_commits.contains(&"D"),
        "Branch-3 commit D must appear (third parent traversal)"
    );

    // Verify expected order
    assert_eq!(bit_commits, vec!["M", "D", "C", "B", "A"]);

    Ok(())
}
