use crate::common::command::{get_head_commit_sha, init_repository_dir, run_bit_command};
use assert_fs::TempDir;
use rstest::rstest;

#[rstest]
fn show_commit_with_decoration_short_oneline(
    init_repository_dir: TempDir,
) -> Result<(), Box<dyn std::error::Error>> {
    let repository_dir = init_repository_dir;

    // Get the commit SHA from the refs
    let expected_commit_sha = get_head_commit_sha(repository_dir.path())?;
    let expected_short_sha = &expected_commit_sha[..7];

    // Run the log command with --oneline and --decorate=short
    let output = run_bit_command(
        repository_dir.path(),
        &["log", "--oneline", "--decorate", "short"],
    )
    .assert()
    .success();

    let stdout = String::from_utf8(output.get_output().stdout.clone())?;

    // Extract the first line
    let first_line = stdout.lines().next().unwrap();

    // Verify that the commit line contains short decorations in oneline format
    // Expected format: "<short-sha> (HEAD -> master) <message>"
    assert!(
        first_line.starts_with(expected_short_sha),
        "Expected line to start with short SHA '{}', but got: {}",
        expected_short_sha,
        first_line
    );

    // Check for short decoration format
    assert!(
        first_line.contains("(HEAD -> master)") || first_line.contains("(HEAD -> main)"),
        "Expected short decoration format '(HEAD -> master)' or '(HEAD -> main)', but got: {}",
        first_line
    );

    Ok(())
}
