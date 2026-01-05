use crate::common::command::{bit_commit, repository_dir, run_bit_command};
use crate::common::file::{FileSpec, write_file};
use assert_fs::TempDir;
use rstest::rstest;

#[rstest]
fn log_filter_by_file_with_revision_range(
    repository_dir: TempDir,
) -> Result<(), Box<dyn std::error::Error>> {
    // This test verifies that file filtering works correctly with revision ranges
    // e.g., `bit log master..feature -- file.txt`

    run_bit_command(repository_dir.path(), &["init"])
        .assert()
        .success();

    // Commit 1 on master: Add file1.txt
    let file1 = FileSpec::new(
        repository_dir.path().join("file1.txt"),
        "initial".to_string(),
    );
    write_file(file1);
    run_bit_command(repository_dir.path(), &["add", "."])
        .assert()
        .success();
    bit_commit(repository_dir.path(), "Initial commit with file1")
        .assert()
        .success();

    // Create feature branch
    run_bit_command(repository_dir.path(), &["branch", "create", "feature"])
        .assert()
        .success();
    run_bit_command(repository_dir.path(), &["checkout", "feature"])
        .assert()
        .success();

    // Commit 2 on feature: Modify file1.txt
    let file1_mod = FileSpec::new(
        repository_dir.path().join("file1.txt"),
        "modified on feature".to_string(),
    );
    write_file(file1_mod);
    run_bit_command(repository_dir.path(), &["add", "."])
        .assert()
        .success();
    bit_commit(repository_dir.path(), "Modify file1 on feature")
        .assert()
        .success();

    // Commit 3 on feature: Add file2.txt
    let file2 = FileSpec::new(
        repository_dir.path().join("file2.txt"),
        "feature file".to_string(),
    );
    write_file(file2);
    run_bit_command(repository_dir.path(), &["add", "."])
        .assert()
        .success();
    bit_commit(repository_dir.path(), "Add file2 on feature")
        .assert()
        .success();

    // Commit 4 on feature: Modify file1.txt again
    let file1_mod2 = FileSpec::new(
        repository_dir.path().join("file1.txt"),
        "modified again on feature".to_string(),
    );
    write_file(file1_mod2);
    run_bit_command(repository_dir.path(), &["add", "."])
        .assert()
        .success();
    bit_commit(repository_dir.path(), "Modify file1 again on feature")
        .assert()
        .success();

    // Test: log master..feature -- file1.txt
    // Should show only commits on feature branch that modified file1.txt (commits 2 and 4)
    let output = run_bit_command(
        repository_dir.path(),
        &[
            "log",
            "master..feature",
            "--decorate=none",
            "--",
            "file1.txt",
        ],
    )
    .assert()
    .success();

    let stdout = String::from_utf8(output.get_output().stdout.clone())?;

    // Should include feature commits that modified file1.txt
    assert!(
        stdout.contains("    Modify file1 on feature"),
        "Expected 'Modify file1 on feature' in filtered log"
    );
    assert!(
        stdout.contains("    Modify file1 again on feature"),
        "Expected 'Modify file1 again on feature' in filtered log"
    );

    // Should NOT include commits that didn't modify file1.txt
    assert!(
        !stdout.contains("    Add file2 on feature"),
        "Should not include 'Add file2 on feature' when filtering by file1.txt"
    );

    // Should NOT include master commits (even though they modified file1.txt)
    assert!(
        !stdout.contains("    Initial commit with file1"),
        "Should not include master commits in master..feature range"
    );

    // Verify the count of commits
    let commit_count = stdout
        .lines()
        .filter(|line| line.starts_with("commit "))
        .count();
    assert_eq!(
        commit_count, 2,
        "Expected exactly 2 commits when filtering master..feature by file1.txt"
    );

    Ok(())
}
