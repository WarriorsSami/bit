use crate::common::command::{bit_commit_with_timestamp, repository_dir, run_bit_command};
use crate::common::file::{FileSpec, write_file};
use assert_fs::TempDir;
use rstest::rstest;

#[rstest]
fn log_excluded_expression_defaults_to_head(
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

    // Add commit on feature branch (T2 = 2023-01-01 12:00:00)
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

    // Switch back to master and add commits
    run_bit_command(repository_dir.path(), &["checkout", "master"])
        .assert()
        .success();

    // Add third commit on master (T3 = 2023-01-01 13:00:00)
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
        "2023-01-01 13:00:00 +0000",
    )
    .assert()
    .success();

    // Test excluded expression without explicit included revision: ^feature
    // This should default to ^feature HEAD (which is master)
    // Expected: Third commit (T3)
    // Excluded: Second commit (T1), Initial commit (T0), Feature commit 1
    let output = run_bit_command(
        repository_dir.path(),
        &["log", "^feature", "--decorate=none"],
    )
    .assert()
    .success();

    let stdout = String::from_utf8(output.get_output().stdout.clone())?;

    // Should include commits only on master after the branch point
    assert!(
        stdout.contains("    Third commit"),
        "Expected 'Third commit' with excluded expression defaulting to HEAD"
    );

    // Should NOT include commits from feature branch
    assert!(
        !stdout.contains("    Feature commit 1"),
        "Should not include 'Feature commit 1' with excluded expression"
    );

    // Should NOT include common ancestor commits
    assert!(
        !stdout.contains("    Second commit"),
        "Should not include 'Second commit' (common ancestor) with excluded expression"
    );
    assert!(
        !stdout.contains("    Initial commit"),
        "Should not include 'Initial commit' (common ancestor) with excluded expression"
    );

    Ok(())
}
