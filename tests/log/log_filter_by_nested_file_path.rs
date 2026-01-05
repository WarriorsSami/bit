use crate::common::command::{bit_commit, repository_dir, run_bit_command};
use crate::common::file::{FileSpec, write_file};
use assert_fs::TempDir;
use rstest::rstest;

#[rstest]
fn log_filter_by_nested_file_path(
    repository_dir: TempDir,
) -> Result<(), Box<dyn std::error::Error>> {
    // This test verifies that filtering by nested file paths works correctly

    run_bit_command(repository_dir.path(), &["init"])
        .assert()
        .success();

    // Commit 1: Add deeply nested file
    let nested_file = FileSpec::new(
        repository_dir
            .path()
            .join("src")
            .join("utils")
            .join("helper.rs"),
        "// helper".to_string(),
    );
    write_file(nested_file);
    run_bit_command(repository_dir.path(), &["add", "."])
        .assert()
        .success();
    bit_commit(repository_dir.path(), "Add helper.rs")
        .assert()
        .success();

    // Commit 2: Add another file in same directory
    let other_file = FileSpec::new(
        repository_dir
            .path()
            .join("src")
            .join("utils")
            .join("config.rs"),
        "// config".to_string(),
    );
    write_file(other_file);
    run_bit_command(repository_dir.path(), &["add", "."])
        .assert()
        .success();
    bit_commit(repository_dir.path(), "Add config.rs")
        .assert()
        .success();

    // Commit 3: Modify the nested file
    let nested_file_mod = FileSpec::new(
        repository_dir
            .path()
            .join("src")
            .join("utils")
            .join("helper.rs"),
        "// helper updated".to_string(),
    );
    write_file(nested_file_mod);
    run_bit_command(repository_dir.path(), &["add", "."])
        .assert()
        .success();
    bit_commit(repository_dir.path(), "Update helper.rs")
        .assert()
        .success();

    // Commit 4: Add file in different nested path
    let different_nested = FileSpec::new(
        repository_dir
            .path()
            .join("tests")
            .join("integration")
            .join("test.rs"),
        "// test".to_string(),
    );
    write_file(different_nested);
    run_bit_command(repository_dir.path(), &["add", "."])
        .assert()
        .success();
    bit_commit(repository_dir.path(), "Add integration test")
        .assert()
        .success();

    // Test: log -- src/utils/helper.rs (should show commits 1 and 3)
    let output = run_bit_command(
        repository_dir.path(),
        &["log", "--decorate=none", "--", "src/utils/helper.rs"],
    )
    .assert()
    .success();

    let stdout = String::from_utf8(output.get_output().stdout.clone())?;

    // Should include commits that modified src/utils/helper.rs
    assert!(
        stdout.contains("    Add helper.rs"),
        "Expected 'Add helper.rs' in filtered log"
    );
    assert!(
        stdout.contains("    Update helper.rs"),
        "Expected 'Update helper.rs' in filtered log"
    );

    // Should NOT include other files
    assert!(
        !stdout.contains("    Add config.rs"),
        "Should not include 'Add config.rs' when filtering by helper.rs"
    );
    assert!(
        !stdout.contains("    Add integration test"),
        "Should not include 'Add integration test' when filtering by helper.rs"
    );

    // Verify the count of commits
    let commit_count = stdout
        .lines()
        .filter(|line| line.starts_with("commit "))
        .count();
    assert_eq!(
        commit_count, 2,
        "Expected exactly 2 commits when filtering by src/utils/helper.rs"
    );

    Ok(())
}
