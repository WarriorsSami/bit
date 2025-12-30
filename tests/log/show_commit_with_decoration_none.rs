use crate::common::command::{get_head_commit_sha, init_repository_dir, run_bit_command};
use assert_fs::TempDir;
use rstest::rstest;

#[rstest]
fn show_commit_with_decoration_none(
    init_repository_dir: TempDir,
) -> Result<(), Box<dyn std::error::Error>> {
    let repository_dir = init_repository_dir;

    // Get the commit SHA from the refs
    let expected_commit_sha = get_head_commit_sha(repository_dir.path())?;

    // Run the log command with --decorate=none
    let output = run_bit_command(repository_dir.path(), &["log", "--decorate", "none"])
        .assert()
        .success();

    let stdout = String::from_utf8(output.get_output().stdout.clone())?;

    // Extract the first line (commit line)
    let first_line = stdout.lines().next().unwrap();

    // Verify that the commit line contains no decorations (no parentheses with refs)
    assert_eq!(
        first_line,
        format!("commit {}", expected_commit_sha),
        "Expected commit line without decorations, but got: {}",
        first_line
    );

    // Ensure no parentheses are present (which would indicate decorations)
    assert!(
        !first_line.contains('(') && !first_line.contains(')'),
        "Expected no decorations, but found parentheses in: {}",
        first_line
    );

    Ok(())
}
