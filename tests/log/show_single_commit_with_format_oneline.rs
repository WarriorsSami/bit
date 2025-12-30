use crate::common::command::{get_head_commit_sha, init_repository_dir, run_bit_command};
use assert_fs::TempDir;
use rstest::rstest;

#[rstest]
fn show_single_commit_with_format_oneline(
    init_repository_dir: TempDir,
) -> Result<(), Box<dyn std::error::Error>> {
    let repository_dir = init_repository_dir;

    // Get the commit SHA from the refs
    let expected_commit_sha = get_head_commit_sha(repository_dir.path())?;

    // Run the log command with --format=oneline flag
    let output = run_bit_command(repository_dir.path(), &["log", "--format=oneline"])
        .assert()
        .success();

    let stdout = String::from_utf8(output.get_output().stdout.clone())?;

    // With --format=oneline, each commit should be on a single line
    // Format: <sha> <commit-message-first-line>
    let lines: Vec<&str> = stdout.trim().lines().collect();

    assert_eq!(
        lines.len(),
        1,
        "Expected 1 commit line in oneline format, got {}:\n{}",
        lines.len(),
        stdout
    );

    let first_line = lines[0];

    // The line should start with an abbreviated SHA (7 chars by default)
    let parts: Vec<&str> = first_line.splitn(2, ' ').collect();
    assert_eq!(
        parts.len(),
        2,
        "Expected line to have SHA and message separated by space, got: {}",
        first_line
    );

    let displayed_sha = parts[0];
    let commit_message = parts[1];

    // Verify the SHA is 40 characters
    assert_eq!(
        displayed_sha.len(),
        40,
        "Expected 40-character SHA, got {} characters: {}",
        displayed_sha.len(),
        displayed_sha
    );

    // Verify it's hexadecimal
    assert!(
        displayed_sha.chars().all(|c| c.is_ascii_hexdigit()),
        "Expected hexadecimal SHA, got: {}",
        displayed_sha
    );

    // Verify the SHA matches the beginning of the full SHA
    assert!(
        expected_commit_sha.starts_with(displayed_sha),
        "SHA '{}' does not match the beginning of full SHA '{}'",
        displayed_sha,
        expected_commit_sha
    );

    // Verify the commit message
    assert_eq!(
        commit_message, "Initial commit",
        "Expected commit message 'Initial commit', got: {}",
        commit_message
    );

    // Verify there's no "Author:" or "Date:" lines (oneline format should be compact)
    assert!(
        !stdout.contains("Author:"),
        "Oneline format should not contain 'Author:' line:\n{}",
        stdout
    );
    assert!(
        !stdout.contains("Date:"),
        "Oneline format should not contain 'Date:' line:\n{}",
        stdout
    );

    Ok(())
}
