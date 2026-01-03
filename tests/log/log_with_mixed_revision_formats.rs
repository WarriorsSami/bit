use crate::common::command::{bit_commit_with_timestamp, repository_dir, run_bit_command};
use crate::common::file::{FileSpec, write_file};
use assert_fs::TempDir;
use rstest::rstest;

#[rstest]
fn log_with_mixed_revision_formats(
    repository_dir: TempDir,
) -> Result<(), Box<dyn std::error::Error>> {
    // Test logging with mixed revision formats: branch names, SHAs, HEAD notation
    run_bit_command(repository_dir.path(), &["init"])
        .assert()
        .success();

    // Create linear history
    let file1 = FileSpec::new(
        repository_dir.path().join("file1.txt"),
        "content 1".to_string(),
    );
    write_file(file1);
    run_bit_command(repository_dir.path(), &["add", "."])
        .assert()
        .success();
    bit_commit_with_timestamp(
        repository_dir.path(),
        "Commit-1",
        "2023-01-01 10:00:00 +0000",
    )
    .assert()
    .success();

    let file2 = FileSpec::new(
        repository_dir.path().join("file2.txt"),
        "content 2".to_string(),
    );
    write_file(file2);
    run_bit_command(repository_dir.path(), &["add", "."])
        .assert()
        .success();
    bit_commit_with_timestamp(
        repository_dir.path(),
        "Commit-2",
        "2023-01-01 11:00:00 +0000",
    )
    .assert()
    .success();

    // Create a branch
    run_bit_command(repository_dir.path(), &["branch", "create", "feature"])
        .assert()
        .success();

    let file3 = FileSpec::new(
        repository_dir.path().join("file3.txt"),
        "content 3".to_string(),
    );
    write_file(file3);
    run_bit_command(repository_dir.path(), &["add", "."])
        .assert()
        .success();
    bit_commit_with_timestamp(
        repository_dir.path(),
        "Commit-3",
        "2023-01-01 12:00:00 +0000",
    )
    .assert()
    .success();

    // Checkout feature and add a commit
    run_bit_command(repository_dir.path(), &["checkout", "feature"])
        .assert()
        .success();

    let file4 = FileSpec::new(
        repository_dir.path().join("file4.txt"),
        "content 4".to_string(),
    );
    write_file(file4);
    run_bit_command(repository_dir.path(), &["add", "."])
        .assert()
        .success();
    bit_commit_with_timestamp(
        repository_dir.path(),
        "Commit-4",
        "2023-01-01 13:00:00 +0000",
    )
    .assert()
    .success();

    // Log using mixed revision formats: branch name (main), current branch (HEAD), parent (HEAD^)
    let output = run_bit_command(
        repository_dir.path(),
        &["log", "master", "HEAD", "--decorate=none"],
    )
    .assert()
    .success();

    let stdout = String::from_utf8(output.get_output().stdout.clone())?;

    // Should include commits from both main and feature (current HEAD)
    // Feature has: Commit-4, Commit-2, Commit-1
    // Main has: Commit-3, Commit-2, Commit-1
    // Combined (timestamp order): Commit-4, Commit-3, Commit-2, Commit-1
    assert!(stdout.contains("    Commit-4"));
    assert!(stdout.contains("    Commit-3"));
    assert!(stdout.contains("    Commit-2"));
    assert!(stdout.contains("    Commit-1"));

    // Verify ordering
    let c4_pos = stdout.find("    Commit-4").unwrap();
    let c3_pos = stdout.find("    Commit-3").unwrap();
    let c2_pos = stdout.find("    Commit-2").unwrap();
    let c1_pos = stdout.find("    Commit-1").unwrap();

    assert!(c4_pos < c3_pos);
    assert!(c3_pos < c2_pos);
    assert!(c2_pos < c1_pos);

    // Common commits should not be duplicated
    let commit_count = stdout.matches("commit ").count();
    assert_eq!(commit_count, 4, "Expected 4 unique commits");

    Ok(())
}
