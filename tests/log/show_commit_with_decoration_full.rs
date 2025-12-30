use crate::common::command::{get_head_commit_sha, init_repository_dir, run_bit_command};
use assert_fs::TempDir;
use rstest::rstest;

#[rstest]
fn show_commit_with_decoration_full(
    init_repository_dir: TempDir,
) -> Result<(), Box<dyn std::error::Error>> {
    let repository_dir = init_repository_dir;

    // Get the commit SHA from the refs
    let expected_commit_sha = get_head_commit_sha(repository_dir.path())?;

    // Run the log command with --decorate=full
    let output = run_bit_command(repository_dir.path(), &["log", "--decorate", "full"])
        .assert()
        .success();

    let stdout = String::from_utf8(output.get_output().stdout.clone())?;

    // Extract the first line (commit line)
    let first_line = stdout.lines().next().unwrap();

    // Verify that the commit line contains full decorations (with refs/heads/ prefix)
    // Expected format: "commit <sha> (HEAD -> refs/heads/master)"
    assert!(
        first_line.starts_with(&format!("commit {}", expected_commit_sha)),
        "Expected commit line to start with 'commit {}', but got: {}",
        expected_commit_sha,
        first_line
    );

    // Check for full decoration format (with refs/heads/ prefix)
    assert!(
        first_line.contains("(HEAD -> refs/heads/master)")
            || first_line.contains("(HEAD -> refs/heads/main)"),
        "Expected full decoration format '(HEAD -> refs/heads/master)' or '(HEAD -> refs/heads/main)', but got: {}",
        first_line
    );

    Ok(())
}
