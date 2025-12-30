use crate::common::command::{get_head_commit_sha, init_repository_dir, run_bit_command};
use assert_fs::TempDir;
use rstest::rstest;

#[rstest]
fn show_commit_with_decoration_short(
    init_repository_dir: TempDir,
) -> Result<(), Box<dyn std::error::Error>> {
    let repository_dir = init_repository_dir;

    // Get the commit SHA from the refs
    let expected_commit_sha = get_head_commit_sha(repository_dir.path())?;

    // Run the log command with --decorate=short (or default)
    let output = run_bit_command(repository_dir.path(), &["log", "--decorate", "short"])
        .assert()
        .success();

    let stdout = String::from_utf8(output.get_output().stdout.clone())?;

    // Extract the first line (commit line)
    let first_line = stdout.lines().next().unwrap();

    // Verify that the commit line contains short decorations (branch names without refs/heads/ prefix)
    // Expected format: "commit <sha> (HEAD -> master)"
    assert!(
        first_line.starts_with(&format!("commit {}", expected_commit_sha)),
        "Expected commit line to start with 'commit {}', but got: {}",
        expected_commit_sha,
        first_line
    );

    // Check for short decoration format (without refs/heads/ prefix)
    assert!(
        first_line.contains("(HEAD -> master)") || first_line.contains("(HEAD -> main)"),
        "Expected short decoration format '(HEAD -> master)' or '(HEAD -> main)', but got: {}",
        first_line
    );

    Ok(())
}
