use crate::common::command::{bit_commit, repository_dir, run_bit_command};
use crate::common::file::{FileSpec, write_file};
use assert_fs::TempDir;
use rstest::rstest;

#[rstest]
fn log_filter_commits_by_multiple_files(
    repository_dir: TempDir,
) -> Result<(), Box<dyn std::error::Error>> {
    // This test verifies that `bit log -- <file1> <file2>` shows commits that modified any of the specified files

    run_bit_command(repository_dir.path(), &["init"])
        .assert()
        .success();

    // Commit 1: Add file1.txt
    let file1 = FileSpec::new(
        repository_dir.path().join("file1.txt"),
        "file1 content".to_string(),
    );
    write_file(file1);
    run_bit_command(repository_dir.path(), &["add", "."])
        .assert()
        .success();
    bit_commit(repository_dir.path(), "Add file1")
        .assert()
        .success();

    // Commit 2: Add file2.txt
    let file2 = FileSpec::new(
        repository_dir.path().join("file2.txt"),
        "file2 content".to_string(),
    );
    write_file(file2);
    run_bit_command(repository_dir.path(), &["add", "."])
        .assert()
        .success();
    bit_commit(repository_dir.path(), "Add file2")
        .assert()
        .success();

    // Commit 3: Add file3.txt (not in filter)
    let file3 = FileSpec::new(
        repository_dir.path().join("file3.txt"),
        "file3 content".to_string(),
    );
    write_file(file3);
    run_bit_command(repository_dir.path(), &["add", "."])
        .assert()
        .success();
    bit_commit(repository_dir.path(), "Add file3")
        .assert()
        .success();

    // Commit 4: Modify file1.txt
    let file1_mod = FileSpec::new(
        repository_dir.path().join("file1.txt"),
        "file1 modified".to_string(),
    );
    write_file(file1_mod);
    run_bit_command(repository_dir.path(), &["add", "."])
        .assert()
        .success();
    bit_commit(repository_dir.path(), "Modify file1")
        .assert()
        .success();

    // Commit 5: Modify file3.txt (not in filter)
    let file3_mod = FileSpec::new(
        repository_dir.path().join("file3.txt"),
        "file3 modified".to_string(),
    );
    write_file(file3_mod);
    run_bit_command(repository_dir.path(), &["add", "."])
        .assert()
        .success();
    bit_commit(repository_dir.path(), "Modify file3")
        .assert()
        .success();

    // Test: log -- file1.txt file2.txt (should show commits 1, 2, and 4)
    let output = run_bit_command(
        repository_dir.path(),
        &["log", "--decorate=none", "--", "file1.txt", "file2.txt"],
    )
    .assert()
    .success();

    let stdout = String::from_utf8(output.get_output().stdout.clone())?;

    // Should include commits that modified file1.txt or file2.txt
    assert!(
        stdout.contains("    Add file1"),
        "Expected 'Add file1' in filtered log"
    );
    assert!(
        stdout.contains("    Add file2"),
        "Expected 'Add file2' in filtered log"
    );
    assert!(
        stdout.contains("    Modify file1"),
        "Expected 'Modify file1' in filtered log"
    );

    // Should NOT include commits that only modified file3.txt
    assert!(
        !stdout.contains("    Add file3"),
        "Should not include 'Add file3' in filtered log"
    );
    assert!(
        !stdout.contains("    Modify file3"),
        "Should not include 'Modify file3' in filtered log"
    );

    // Verify the count of commits
    let commit_count = stdout
        .lines()
        .filter(|line| line.starts_with("commit "))
        .count();
    assert_eq!(
        commit_count, 3,
        "Expected exactly 3 commits when filtering by file1.txt and file2.txt"
    );

    Ok(())
}
