use crate::common::command::{bit_commit, repository_dir, run_bit_command};
use crate::common::file::{FileSpec, write_file};
use assert_fs::TempDir;
use rstest::rstest;

#[rstest]
fn log_filter_commits_by_single_file(
    repository_dir: TempDir,
) -> Result<(), Box<dyn std::error::Error>> {
    // This test verifies that `bit log -- <file>` only shows commits that modified the specified file

    run_bit_command(repository_dir.path(), &["init"])
        .assert()
        .success();

    // Commit 1: Add file1.txt
    let file1 = FileSpec::new(
        repository_dir.path().join("file1.txt"),
        "initial content".to_string(),
    );
    write_file(file1);
    run_bit_command(repository_dir.path(), &["add", "."])
        .assert()
        .success();
    bit_commit(repository_dir.path(), "Add file1.txt")
        .assert()
        .success();

    // Commit 2: Add file2.txt (different file)
    let file2 = FileSpec::new(
        repository_dir.path().join("file2.txt"),
        "other content".to_string(),
    );
    write_file(file2);
    run_bit_command(repository_dir.path(), &["add", "."])
        .assert()
        .success();
    bit_commit(repository_dir.path(), "Add file2.txt")
        .assert()
        .success();

    // Commit 3: Modify file1.txt
    let file1_modified = FileSpec::new(
        repository_dir.path().join("file1.txt"),
        "modified content".to_string(),
    );
    write_file(file1_modified);
    run_bit_command(repository_dir.path(), &["add", "."])
        .assert()
        .success();
    bit_commit(repository_dir.path(), "Modify file1.txt")
        .assert()
        .success();

    // Commit 4: Modify file2.txt
    let file2_modified = FileSpec::new(
        repository_dir.path().join("file2.txt"),
        "other modified content".to_string(),
    );
    write_file(file2_modified);
    run_bit_command(repository_dir.path(), &["add", "."])
        .assert()
        .success();
    bit_commit(repository_dir.path(), "Modify file2.txt")
        .assert()
        .success();

    // Test: log -- file1.txt (should show only commits 1 and 3)
    let output = run_bit_command(
        repository_dir.path(),
        &["log", "--decorate=none", "--", "file1.txt"],
    )
    .assert()
    .success();

    let stdout = String::from_utf8(output.get_output().stdout.clone())?;
    println!("Filtered log output:\n{}", stdout);

    // Should include commits that modified file1.txt
    assert!(
        stdout.contains("    Modify file1.txt"),
        "Expected 'Modify file1.txt' in filtered log"
    );
    assert!(
        stdout.contains("    Add file1.txt"),
        "Expected 'Add file1.txt' in filtered log"
    );

    // Should NOT include commits that only modified file2.txt
    assert!(
        !stdout.contains("    Add file2.txt"),
        "Should not include 'Add file2.txt' in filtered log for file1.txt"
    );
    assert!(
        !stdout.contains("    Modify file2.txt"),
        "Should not include 'Modify file2.txt' in filtered log for file1.txt"
    );

    // Verify the count of commits
    let commit_count = stdout
        .lines()
        .filter(|line| line.starts_with("commit "))
        .count();
    assert_eq!(
        commit_count, 2,
        "Expected exactly 2 commits when filtering by file1.txt"
    );

    Ok(())
}
