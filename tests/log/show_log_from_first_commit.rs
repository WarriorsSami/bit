use crate::common::command::{
    bit_commit, get_head_commit_sha, get_parent_commit_id, run_bit_command,
};
use crate::common::file::{FileSpec, write_file};
use assert_fs::TempDir;
use rstest::rstest;

#[rstest]
fn show_log_from_first_commit(
    #[from(crate::common::command::repository_dir)] repository_dir: TempDir,
) -> Result<(), Box<dyn std::error::Error>> {
    // Initialize repository
    run_bit_command(repository_dir.path(), &["init"])
        .assert()
        .success();

    // Create a linear history with 5 commits
    for i in 1..=5 {
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

    // Get the first commit SHA
    let commit5_sha = get_head_commit_sha(repository_dir.path())?;
    let commit4_sha = get_parent_commit_id(repository_dir.path(), &commit5_sha)?;
    let commit3_sha = get_parent_commit_id(repository_dir.path(), &commit4_sha)?;
    let commit2_sha = get_parent_commit_id(repository_dir.path(), &commit3_sha)?;
    let commit1_sha = get_parent_commit_id(repository_dir.path(), &commit2_sha)?;

    // Run the log command starting from the first commit
    let output = run_bit_command(
        repository_dir.path(),
        &["log", &commit1_sha, "--decorate=none"],
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

    // Verify we have exactly 1 commit (only the first commit)
    assert_eq!(
        commit_shas.len(),
        1,
        "Expected 1 commit in output when starting from first commit, found {}:\n{}",
        commit_shas.len(),
        stdout
    );

    // Verify it's the first commit
    assert_eq!(
        commit_shas[0], commit1_sha,
        "Should only show the first commit"
    );

    // Verify the commit message
    assert!(
        stdout.contains("Commit 1"),
        "Expected 'Commit 1' message in output"
    );

    // Verify later commits are NOT in the output
    assert!(
        !stdout.contains(&commit2_sha),
        "Commit 2 should not appear in output"
    );
    assert!(
        !stdout.contains(&commit3_sha),
        "Commit 3 should not appear in output"
    );
    assert!(
        !stdout.contains(&commit4_sha),
        "Commit 4 should not appear in output"
    );
    assert!(
        !stdout.contains(&commit5_sha),
        "Commit 5 should not appear in output"
    );

    Ok(())
}
