use crate::common::command::{bit_commit_with_timestamp, repository_dir, run_bit_command};
use crate::common::file::{FileSpec, write_file};
use assert_fs::TempDir;
use rstest::rstest;

#[rstest]
fn log_multiple_excluded_revisions(
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

    // Create feature-a branch from second commit
    run_bit_command(repository_dir.path(), &["branch", "create", "feature-a"])
        .assert()
        .success();
    run_bit_command(repository_dir.path(), &["checkout", "feature-a"])
        .assert()
        .success();

    // Add commit on feature-a branch (T2 = 2023-01-01 12:00:00)
    let file_a1 = FileSpec::new(
        repository_dir.path().join("feature_a1.txt"),
        "feature-a commit 1".to_string(),
    );
    write_file(file_a1);
    run_bit_command(repository_dir.path(), &["add", "."])
        .assert()
        .success();
    bit_commit_with_timestamp(
        repository_dir.path(),
        "Feature A - Commit 1",
        "2023-01-01 12:00:00 +0000",
    )
    .assert()
    .success();

    // Switch back to master and create feature-b branch
    run_bit_command(repository_dir.path(), &["checkout", "master"])
        .assert()
        .success();
    run_bit_command(repository_dir.path(), &["branch", "create", "feature-b"])
        .assert()
        .success();
    run_bit_command(repository_dir.path(), &["checkout", "feature-b"])
        .assert()
        .success();

    // Add commit on feature-b branch (T3 = 2023-01-01 13:00:00)
    let file_b1 = FileSpec::new(
        repository_dir.path().join("feature_b1.txt"),
        "feature-b commit 1".to_string(),
    );
    write_file(file_b1);
    run_bit_command(repository_dir.path(), &["add", "."])
        .assert()
        .success();
    bit_commit_with_timestamp(
        repository_dir.path(),
        "Feature B - Commit 1",
        "2023-01-01 13:00:00 +0000",
    )
    .assert()
    .success();

    // Switch back to master and add commits
    run_bit_command(repository_dir.path(), &["checkout", "master"])
        .assert()
        .success();

    // Add third commit on master (T4 = 2023-01-01 14:00:00)
    let file3 = FileSpec::new(
        repository_dir.path().join("file3.txt"),
        "third commit".to_string(),
    );
    write_file(file3);
    run_bit_command(repository_dir.path(), &["add", "."])
        .assert()
        .success();
    bit_commit_with_timestamp(
        repository_dir.path(),
        "Third commit",
        "2023-01-01 14:00:00 +0000",
    )
    .assert()
    .success();

    // Add fourth commit on master (T5 = 2023-01-01 15:00:00)
    let file4 = FileSpec::new(
        repository_dir.path().join("file4.txt"),
        "fourth commit".to_string(),
    );
    write_file(file4);
    run_bit_command(repository_dir.path(), &["add", "."])
        .assert()
        .success();
    bit_commit_with_timestamp(
        repository_dir.path(),
        "Fourth commit",
        "2023-01-01 15:00:00 +0000",
    )
    .assert()
    .success();

    // Test multiple excluded expressions: ^feature-a ^feature-b master
    // This should show commits reachable from master but NOT from feature-a or feature-b
    // Expected: Fourth commit (T5), Third commit (T4)
    // Excluded: Second commit (T1), Initial commit (T0), Feature-A and Feature-B commits
    let output = run_bit_command(
        repository_dir.path(),
        &[
            "log",
            "^feature-a",
            "^feature-b",
            "master",
            "--decorate=none",
        ],
    )
    .assert()
    .success();

    let stdout = String::from_utf8(output.get_output().stdout.clone())?;

    // Should include commits only on master after the branch point
    assert!(
        stdout.contains("    Fourth commit"),
        "Expected 'Fourth commit' with multiple excluded expressions"
    );
    assert!(
        stdout.contains("    Third commit"),
        "Expected 'Third commit' with multiple excluded expressions"
    );

    // Should NOT include commits from feature-a branch
    assert!(
        !stdout.contains("    Feature A - Commit 1"),
        "Should not include 'Feature A - Commit 1' with multiple excluded expressions"
    );

    // Should NOT include commits from feature-b branch
    assert!(
        !stdout.contains("    Feature B - Commit 1"),
        "Should not include 'Feature B - Commit 1' with multiple excluded expressions"
    );

    // Should NOT include common ancestor commits
    assert!(
        !stdout.contains("    Second commit"),
        "Should not include 'Second commit' (common ancestor) with multiple excluded expressions"
    );
    assert!(
        !stdout.contains("    Initial commit"),
        "Should not include 'Initial commit' (common ancestor) with multiple excluded expressions"
    );

    // Verify commits are in timestamp order (newest first)
    let fourth_pos = stdout.find("    Fourth commit").unwrap();
    let third_pos = stdout.find("    Third commit").unwrap();
    assert!(
        fourth_pos < third_pos,
        "Commits should be ordered by timestamp (newest first)"
    );

    Ok(())
}
