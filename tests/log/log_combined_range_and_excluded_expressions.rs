use crate::common::command::{bit_commit_with_timestamp, repository_dir, run_bit_command};
use crate::common::file::{FileSpec, write_file};
use assert_fs::TempDir;
use rstest::rstest;

#[rstest]
fn log_combined_range_and_excluded_expressions(
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

    // Add commits on feature-a branch (T2 = 2023-01-01 12:00:00)
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

    // Add second commit on feature-a (T3 = 2023-01-01 13:00:00)
    let file_a2 = FileSpec::new(
        repository_dir.path().join("feature_a2.txt"),
        "feature-a commit 2".to_string(),
    );
    write_file(file_a2);
    run_bit_command(repository_dir.path(), &["add", "."])
        .assert()
        .success();
    bit_commit_with_timestamp(
        repository_dir.path(),
        "Feature A - Commit 2",
        "2023-01-01 13:00:00 +0000",
    )
    .assert()
    .success();

    // Create feature-b branch from feature-a's first commit
    run_bit_command(repository_dir.path(), &["checkout", "master"])
        .assert()
        .success();
    run_bit_command(repository_dir.path(), &["branch", "create", "feature-b"])
        .assert()
        .success();
    run_bit_command(repository_dir.path(), &["checkout", "feature-b"])
        .assert()
        .success();

    // Add commit on feature-b (T4 = 2023-01-01 14:00:00)
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
        "2023-01-01 14:00:00 +0000",
    )
    .assert()
    .success();

    // Switch back to master and add commits
    run_bit_command(repository_dir.path(), &["checkout", "master"])
        .assert()
        .success();

    // Add third commit on master (T5 = 2023-01-01 15:00:00)
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
        "2023-01-01 15:00:00 +0000",
    )
    .assert()
    .success();

    // Add fourth commit on master (T6 = 2023-01-01 16:00:00)
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
        "2023-01-01 16:00:00 +0000",
    )
    .assert()
    .success();

    // Test combining range and excluded expressions: ^feature-b master..feature-a
    // Range: master..feature-a shows commits in feature-a not in master
    //        That would be: Feature A - Commit 2, Feature A - Commit 1
    // Excluded: ^feature-b excludes commits reachable from feature-b
    //           feature-b has: Second commit, Initial commit
    // Combined result: Feature A - Commit 2, Feature A - Commit 1
    // (Since feature-b doesn't have the feature-a commits, they won't be excluded)
    let output = run_bit_command(
        repository_dir.path(),
        &["log", "^feature-b", "master..feature-a", "--decorate=none"],
    )
    .assert()
    .success();

    let stdout = String::from_utf8(output.get_output().stdout.clone())?;

    // Should include feature-a commits
    assert!(
        stdout.contains("    Feature A - Commit 2"),
        "Expected 'Feature A - Commit 2' with combined expressions"
    );
    assert!(
        stdout.contains("    Feature A - Commit 1"),
        "Expected 'Feature A - Commit 1' with combined expressions"
    );

    // Should NOT include master commits
    assert!(
        !stdout.contains("    Fourth commit"),
        "Should not include 'Fourth commit' (on master) with combined expressions"
    );
    assert!(
        !stdout.contains("    Third commit"),
        "Should not include 'Third commit' (on master) with combined expressions"
    );

    // Should NOT include feature-b commits
    assert!(
        !stdout.contains("    Feature B - Commit 1"),
        "Should not include 'Feature B - Commit 1' with combined expressions"
    );

    // Should NOT include common ancestor commits
    assert!(
        !stdout.contains("    Second commit"),
        "Should not include 'Second commit' (common ancestor) with combined expressions"
    );
    assert!(
        !stdout.contains("    Initial commit"),
        "Should not include 'Initial commit' (common ancestor) with combined expressions"
    );

    Ok(())
}
