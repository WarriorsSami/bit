use crate::common::command::{
    bit_commit, get_head_commit_sha, get_parent_commit_id, run_bit_command,
};
use crate::common::file::{FileSpec, write_file};
use assert_fs::TempDir;
use rstest::rstest;

#[rstest]
fn show_log_from_branch_reference(
    #[from(crate::common::command::repository_dir)] repository_dir: TempDir,
) -> Result<(), Box<dyn std::error::Error>> {
    // Initialize repository
    run_bit_command(repository_dir.path(), &["init"])
        .assert()
        .success();

    // Create initial commits on master branch
    for i in 1..=3 {
        let file = FileSpec::new(
            repository_dir.path().join(format!("main{}.txt", i)),
            format!("Main content {}", i),
        );
        write_file(file);
        run_bit_command(repository_dir.path(), &["add", "."])
            .assert()
            .success();
        bit_commit(repository_dir.path(), &format!("Main commit {}", i))
            .assert()
            .success();
    }

    // Get the current HEAD SHA (main commit 3)
    let main_commit3_sha = get_head_commit_sha(repository_dir.path())?;
    let main_commit2_sha = get_parent_commit_id(repository_dir.path(), &main_commit3_sha)?;

    // Create a feature branch from commit 2
    run_bit_command(
        repository_dir.path(),
        &["branch", "create", "feature", &main_commit2_sha],
    )
    .assert()
    .success();

    // Checkout the feature branch
    run_bit_command(repository_dir.path(), &["checkout", "feature"])
        .assert()
        .success();

    // Add commits on the feature branch
    for i in 1..=2 {
        let file = FileSpec::new(
            repository_dir.path().join(format!("feature{}.txt", i)),
            format!("Feature content {}", i),
        );
        write_file(file);
        run_bit_command(repository_dir.path(), &["add", "."])
            .assert()
            .success();
        bit_commit(repository_dir.path(), &format!("Feature commit {}", i))
            .assert()
            .success();
    }

    // Run log from the main branch reference
    let output = run_bit_command(repository_dir.path(), &["log", "master", "--decorate=none"])
        .assert()
        .success();

    let stdout = String::from_utf8(output.get_output().stdout.clone())?;

    // Extract all commit SHAs from the output
    let commit_shas: Vec<&str> = stdout
        .lines()
        .filter(|line| line.starts_with("commit "))
        .map(|line| line.strip_prefix("commit ").unwrap())
        .collect();

    // Verify we have exactly 3 commits from master branch
    assert_eq!(
        commit_shas.len(),
        3,
        "Expected 3 commits from master branch, found {}:\n{}",
        commit_shas.len(),
        stdout
    );

    // Verify commit messages from master branch are present
    assert!(
        stdout.contains("Main commit 3"),
        "Expected 'Main commit 3' message in output"
    );
    assert!(
        stdout.contains("Main commit 2"),
        "Expected 'Main commit 2' message in output"
    );
    assert!(
        stdout.contains("Main commit 1"),
        "Expected 'Main commit 1' message in output"
    );

    // Verify feature branch commits are NOT in the output
    assert!(
        !stdout.contains("Feature commit 1"),
        "Should not contain 'Feature commit 1' message"
    );
    assert!(
        !stdout.contains("Feature commit 2"),
        "Should not contain 'Feature commit 2' message"
    );

    Ok(())
}
