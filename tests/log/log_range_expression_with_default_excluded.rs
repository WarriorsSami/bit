use crate::common::command::{bit_commit_with_timestamp, repository_dir, run_bit_command};
use crate::common::file::{FileSpec, write_file};
use assert_fs::TempDir;
use rstest::rstest;

#[rstest]
fn log_range_expression_with_default_excluded(
    repository_dir: TempDir,
) -> Result<(), Box<dyn std::error::Error>> {
    // Initialize repository
    run_bit_command(repository_dir.path(), &["init"])
        .assert()
        .success();

    // Create initial commit on master (T0 = 2023-01-01 10:00:00)
    let file1 = FileSpec::new(
        repository_dir.path().join("file1.txt"),
        "initial content".to_string(),
    );
    write_file(file1);
    run_bit_command(repository_dir.path(), &["add", "."])
        .assert()
        .success();
    bit_commit_with_timestamp(
        repository_dir.path(),
        "Initial commit",
        "2023-01-01 10:00:00 +0000",
    )
    .assert()
    .success();

    // Create second commit on master (T1 = 2023-01-01 11:00:00)
    let file2 = FileSpec::new(
        repository_dir.path().join("file2.txt"),
        "second commit".to_string(),
    );
    write_file(file2);
    run_bit_command(repository_dir.path(), &["add", "."])
        .assert()
        .success();
    bit_commit_with_timestamp(
        repository_dir.path(),
        "Second commit",
        "2023-01-01 11:00:00 +0000",
    )
    .assert()
    .success();

    // Create feature branch from second commit
    run_bit_command(repository_dir.path(), &["branch", "create", "feature"])
        .assert()
        .success();
    run_bit_command(repository_dir.path(), &["checkout", "feature"])
        .assert()
        .success();

    // Add commits on feature branch (T2 = 2023-01-01 12:00:00)
    let file_f1 = FileSpec::new(
        repository_dir.path().join("feature1.txt"),
        "feature commit 1".to_string(),
    );
    write_file(file_f1);
    run_bit_command(repository_dir.path(), &["add", "."])
        .assert()
        .success();
    bit_commit_with_timestamp(
        repository_dir.path(),
        "Feature commit 1",
        "2023-01-01 12:00:00 +0000",
    )
    .assert()
    .success();

    // Add second commit on feature branch (T3 = 2023-01-01 13:00:00)
    let file_f2 = FileSpec::new(
        repository_dir.path().join("feature2.txt"),
        "feature commit 2".to_string(),
    );
    write_file(file_f2);
    run_bit_command(repository_dir.path(), &["add", "."])
        .assert()
        .success();
    bit_commit_with_timestamp(
        repository_dir.path(),
        "Feature commit 2",
        "2023-01-01 13:00:00 +0000",
    )
    .assert()
    .success();

    // Test range expression with empty excluded: ..feature
    // This should default to HEAD..feature
    // Since we're on feature branch, HEAD == feature, so this should show no commits
    // (all commits reachable from feature are also reachable from HEAD)
    let output = run_bit_command(
        repository_dir.path(),
        &["log", "..feature", "--decorate=none"],
    )
    .assert()
    .success();

    let stdout = String::from_utf8(output.get_output().stdout.clone())?;

    // Since HEAD == feature, no commits should be shown
    // (no commits reachable from feature are excluded by HEAD)
    assert!(
        stdout.contains("No commits to show.") || stdout.trim().is_empty(),
        "Expected no commits when excluded (HEAD) equals included (feature)"
    );

    // Now switch to master and test again
    run_bit_command(repository_dir.path(), &["checkout", "master"])
        .assert()
        .success();

    // Test range expression with empty excluded: ..feature
    // This should default to HEAD..feature (master..feature)
    // Expected: Feature commit 2 (T3), Feature commit 1 (T2)
    let output = run_bit_command(
        repository_dir.path(),
        &["log", "..feature", "--decorate=none"],
    )
    .assert()
    .success();

    let stdout = String::from_utf8(output.get_output().stdout.clone())?;

    // Should include commits only on feature branch after the branch point
    assert!(
        stdout.contains("    Feature commit 2"),
        "Expected 'Feature commit 2' in range output with default excluded (HEAD)"
    );
    assert!(
        stdout.contains("    Feature commit 1"),
        "Expected 'Feature commit 1' in range output with default excluded (HEAD)"
    );

    // Should NOT include common ancestor commits
    assert!(
        !stdout.contains("    Second commit"),
        "Should not include 'Second commit' (common ancestor) in range output"
    );
    assert!(
        !stdout.contains("    Initial commit"),
        "Should not include 'Initial commit' (common ancestor) in range output"
    );

    // Verify commits are in timestamp order (newest first)
    let commit2_pos = stdout.find("    Feature commit 2").unwrap();
    let commit1_pos = stdout.find("    Feature commit 1").unwrap();
    assert!(
        commit2_pos < commit1_pos,
        "Commits should be ordered by timestamp (newest first)"
    );

    Ok(())
}
