use crate::common::command::{bit_commit, repository_dir, run_bit_command};
use crate::common::file::{FileSpec, write_file};
use assert_fs::TempDir;
use rstest::rstest;

#[rstest]
fn log_filter_commits_by_directory(
    repository_dir: TempDir,
) -> Result<(), Box<dyn std::error::Error>> {
    // This test verifies that `bit log -- <dir>/` shows commits that modified any file within the directory

    run_bit_command(repository_dir.path(), &["init"])
        .assert()
        .success();

    // Commit 1: Add file in src directory
    let src_file1 = FileSpec::new(
        repository_dir.path().join("src").join("main.rs"),
        "fn main() {}".to_string(),
    );
    write_file(src_file1);
    run_bit_command(repository_dir.path(), &["add", "."])
        .assert()
        .success();
    bit_commit(repository_dir.path(), "Add src/main.rs")
        .assert()
        .success();

    // Commit 2: Add file in docs directory
    let docs_file = FileSpec::new(
        repository_dir.path().join("docs").join("README.md"),
        "# Docs".to_string(),
    );
    write_file(docs_file);
    run_bit_command(repository_dir.path(), &["add", "."])
        .assert()
        .success();
    bit_commit(repository_dir.path(), "Add docs/README.md")
        .assert()
        .success();

    // Commit 3: Add another file in src directory
    let src_file2 = FileSpec::new(
        repository_dir.path().join("src").join("lib.rs"),
        "// lib".to_string(),
    );
    write_file(src_file2);
    run_bit_command(repository_dir.path(), &["add", "."])
        .assert()
        .success();
    bit_commit(repository_dir.path(), "Add src/lib.rs")
        .assert()
        .success();

    // Commit 4: Modify file in docs directory
    let docs_file_mod = FileSpec::new(
        repository_dir.path().join("docs").join("README.md"),
        "# Updated Docs".to_string(),
    );
    write_file(docs_file_mod);
    run_bit_command(repository_dir.path(), &["add", "."])
        .assert()
        .success();
    bit_commit(repository_dir.path(), "Update docs/README.md")
        .assert()
        .success();

    // Commit 5: Add root-level file
    let root_file = FileSpec::new(
        repository_dir.path().join("README.md"),
        "# Project".to_string(),
    );
    write_file(root_file);
    run_bit_command(repository_dir.path(), &["add", "."])
        .assert()
        .success();
    bit_commit(repository_dir.path(), "Add README.md at root")
        .assert()
        .success();

    // Test: log -- src/ (should show commits 1 and 3)
    let output = run_bit_command(
        repository_dir.path(),
        &["log", "--decorate=none", "--", "src/"],
    )
    .assert()
    .success();

    let stdout = String::from_utf8(output.get_output().stdout.clone())?;

    // Should include commits that modified files in src/
    assert!(
        stdout.contains("    Add src/main.rs"),
        "Expected 'Add src/main.rs' in filtered log"
    );
    assert!(
        stdout.contains("    Add src/lib.rs"),
        "Expected 'Add src/lib.rs' in filtered log"
    );

    // Should NOT include commits in other directories
    assert!(
        !stdout.contains("    Add docs/README.md"),
        "Should not include 'Add docs/README.md' in filtered log for src/"
    );
    assert!(
        !stdout.contains("    Update docs/README.md"),
        "Should not include 'Update docs/README.md' in filtered log for src/"
    );
    assert!(
        !stdout.contains("    Add README.md at root"),
        "Should not include 'Add README.md at root' in filtered log for src/"
    );

    // Verify the count of commits
    let commit_count = stdout
        .lines()
        .filter(|line| line.starts_with("commit "))
        .count();
    assert_eq!(
        commit_count, 2,
        "Expected exactly 2 commits when filtering by src/ directory"
    );

    Ok(())
}
