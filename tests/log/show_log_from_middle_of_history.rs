use crate::common::command::{
    bit_commit, get_head_commit_sha, get_parent_commit_id, run_bit_command,
};
use crate::common::file::{FileSpec, write_file};
use assert_fs::TempDir;
use rstest::rstest;

#[rstest]
fn show_log_from_middle_of_history(
    #[from(crate::common::command::repository_dir)] repository_dir: TempDir,
) -> Result<(), Box<dyn std::error::Error>> {
    // Initialize repository
    run_bit_command(repository_dir.path(), &["init"])
        .assert()
        .success();

    // Create a linear history with 7 commits
    let commit_messages = [
        "Initial setup",
        "Add configuration",
        "Implement feature A",
        "Add tests for feature A",
        "Implement feature B",
        "Add tests for feature B",
        "Update documentation",
    ];

    for (i, message) in commit_messages.iter().enumerate() {
        let file = FileSpec::new(
            repository_dir.path().join(format!("file{}.txt", i + 1)),
            format!("Content for {}", message),
        );
        write_file(file);
        run_bit_command(repository_dir.path(), &["add", "."])
            .assert()
            .success();
        bit_commit(repository_dir.path(), message)
            .assert()
            .success();
    }

    // Get commit SHAs (walking back from HEAD)
    let commit7_sha = get_head_commit_sha(repository_dir.path())?;
    let commit6_sha = get_parent_commit_id(repository_dir.path(), &commit7_sha)?;
    let commit5_sha = get_parent_commit_id(repository_dir.path(), &commit6_sha)?;
    let commit4_sha = get_parent_commit_id(repository_dir.path(), &commit5_sha)?;
    let commit3_sha = get_parent_commit_id(repository_dir.path(), &commit4_sha)?;
    let commit2_sha = get_parent_commit_id(repository_dir.path(), &commit3_sha)?;
    let commit1_sha = get_parent_commit_id(repository_dir.path(), &commit2_sha)?;

    // Run the log command starting from commit 4 (middle of history)
    let output = run_bit_command(
        repository_dir.path(),
        &["log", &commit4_sha, "--decorate=none"],
    )
    .assert()
    .success();

    let stdout = String::from_utf8(output.get_output().stdout.clone())?;

    // Extract all commit SHAs from the output
    let commit_shas: Vec<&str> = stdout
        .lines()
        .filter(|line| line.starts_with("commit "))
        .map(|line| line.strip_prefix("commit ").unwrap())
        .collect();

    // Verify we have exactly 4 commits (commit4 through commit1)
    assert_eq!(
        commit_shas.len(),
        4,
        "Expected 4 commits in output when starting from commit 4, found {}:\n{}",
        commit_shas.len(),
        stdout
    );

    // Verify the commits are in the correct order
    let expected_shas = [&commit4_sha, &commit3_sha, &commit2_sha, &commit1_sha];
    for (i, (actual, expected)) in commit_shas.iter().zip(expected_shas.iter()).enumerate() {
        assert_eq!(
            actual,
            expected,
            "Commit {} mismatch. Expected: {}, Got: {}",
            i + 1,
            expected,
            actual
        );
    }

    // Verify expected commit messages are present
    assert!(
        stdout.contains("Add tests for feature A"),
        "Expected 'Add tests for feature A' message"
    );
    assert!(
        stdout.contains("Implement feature A"),
        "Expected 'Implement feature A' message"
    );
    assert!(
        stdout.contains("Add configuration"),
        "Expected 'Add configuration' message"
    );
    assert!(
        stdout.contains("Initial setup"),
        "Expected 'Initial setup' message"
    );

    // Verify later commits are NOT in the output
    assert!(
        !stdout.contains("Implement feature B"),
        "Should not contain 'Implement feature B' message"
    );
    assert!(
        !stdout.contains("Add tests for feature B"),
        "Should not contain 'Add tests for feature B' message"
    );
    assert!(
        !stdout.contains("Update documentation"),
        "Should not contain 'Update documentation' message"
    );

    Ok(())
}
