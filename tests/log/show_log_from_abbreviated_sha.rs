use crate::common::command::{
    bit_commit, get_head_commit_sha, get_parent_commit_id, run_bit_command,
};
use crate::common::file::{FileSpec, write_file};
use assert_fs::TempDir;
use rstest::rstest;

#[rstest]
fn show_log_from_abbreviated_sha(
    #[from(crate::common::command::repository_dir)] repository_dir: TempDir,
) -> Result<(), Box<dyn std::error::Error>> {
    // Initialize repository
    run_bit_command(repository_dir.path(), &["init"])
        .assert()
        .success();

    // Create a linear history with 4 commits
    for i in 1..=4 {
        let file = FileSpec::new(
            repository_dir.path().join(format!("file{}.txt", i)),
            format!("Content {}", i),
        );
        write_file(file);
        run_bit_command(repository_dir.path(), &["add", "."])
            .assert()
            .success();
        bit_commit(repository_dir.path(), &format!("Commit {}", i))
            .assert()
            .success();
    }

    // Get commit SHAs
    let commit4_sha = get_head_commit_sha(repository_dir.path())?;
    let commit3_sha = get_parent_commit_id(repository_dir.path(), &commit4_sha)?;
    let commit2_sha = get_parent_commit_id(repository_dir.path(), &commit3_sha)?;
    let commit1_sha = get_parent_commit_id(repository_dir.path(), &commit2_sha)?;

    // Use an abbreviated SHA (first 7 characters) of commit 3
    let abbreviated_sha = &commit3_sha[0..7];

    // Run the log command starting from abbreviated SHA
    let output = run_bit_command(
        repository_dir.path(),
        &["log", abbreviated_sha, "--decorate=none"],
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
        "Expected 3 commits in output when starting from abbreviated SHA, found {}:\n{}",
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

    // Verify commit 4 is NOT in the output
    assert!(
        !stdout.contains(&commit4_sha),
        "Commit 4 should not appear in output"
    );

    // Verify commit messages
    assert!(
        stdout.contains("Commit 3"),
        "Expected 'Commit 3' message in output"
    );
    assert!(
        stdout.contains("Commit 2"),
        "Expected 'Commit 2' message in output"
    );
    assert!(
        stdout.contains("Commit 1"),
        "Expected 'Commit 1' message in output"
    );
    assert!(
        !stdout.contains("Commit 4"),
        "Should not contain 'Commit 4' message"
    );

    Ok(())
}
