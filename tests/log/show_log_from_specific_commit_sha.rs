use crate::common::command::{
    bit_commit, get_head_commit_sha, get_parent_commit_id, run_bit_command,
};
use crate::common::file::{FileSpec, write_file};
use assert_fs::TempDir;
use rstest::rstest;

#[rstest]
fn show_log_from_specific_commit_sha(
    #[from(crate::common::command::repository_dir)] repository_dir: TempDir,
) -> Result<(), Box<dyn std::error::Error>> {
    // Initialize repository
    run_bit_command(repository_dir.path(), &["init"])
        .assert()
        .success();

    // Create a linear history with 5 commits
    let commit_messages = [
        "First commit",
        "Second commit",
        "Third commit",
        "Fourth commit",
        "Fifth commit",
    ];

    for (i, message) in commit_messages.iter().enumerate() {
        let file = FileSpec::new(
            repository_dir.path().join(format!("file{}.txt", i + 1)),
            format!("Content for file {}", i + 1),
        );
        write_file(file);
        run_bit_command(repository_dir.path(), &["add", "."])
            .assert()
            .success();
        bit_commit(repository_dir.path(), message)
            .assert()
            .success();
    }

    // Get commit SHAs
    let commit5_sha = get_head_commit_sha(repository_dir.path())?;
    let commit4_sha = get_parent_commit_id(repository_dir.path(), &commit5_sha)?;
    let commit3_sha = get_parent_commit_id(repository_dir.path(), &commit4_sha)?;
    let commit2_sha = get_parent_commit_id(repository_dir.path(), &commit3_sha)?;
    let commit1_sha = get_parent_commit_id(repository_dir.path(), &commit2_sha)?;

    // Run the log command starting from commit 3
    let output = run_bit_command(
        repository_dir.path(),
        &["log", &commit3_sha, "--decorate=none"],
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

    // Verify we have exactly 3 commits (commit3, commit2, commit1)
    assert_eq!(
        commit_shas.len(),
        3,
        "Expected 3 commits in output when starting from commit 3, found {}:\n{}",
        commit_shas.len(),
        stdout
    );

    // Verify the commits are in the correct order
    assert_eq!(
        commit_shas[0], commit3_sha,
        "First commit should be commit3"
    );
    assert_eq!(
        commit_shas[1], commit2_sha,
        "Second commit should be commit2"
    );
    assert_eq!(
        commit_shas[2], commit1_sha,
        "Third commit should be commit1"
    );

    // Verify that commits 4 and 5 are NOT in the output
    assert!(
        !stdout.contains(&commit4_sha),
        "Commit 4 should not appear in output"
    );
    assert!(
        !stdout.contains(&commit5_sha),
        "Commit 5 should not appear in output"
    );

    // Verify commit messages
    assert!(
        stdout.contains("Third commit"),
        "Expected 'Third commit' message in output"
    );
    assert!(
        stdout.contains("Second commit"),
        "Expected 'Second commit' message in output"
    );
    assert!(
        stdout.contains("First commit"),
        "Expected 'First commit' message in output"
    );
    assert!(
        !stdout.contains("Fourth commit"),
        "Should not contain 'Fourth commit' message"
    );
    assert!(
        !stdout.contains("Fifth commit"),
        "Should not contain 'Fifth commit' message"
    );

    Ok(())
}
