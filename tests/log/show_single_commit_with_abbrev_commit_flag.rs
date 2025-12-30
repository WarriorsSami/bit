use crate::common::command::{get_head_commit_sha, init_repository_dir, run_bit_command};
use assert_fs::TempDir;
use rstest::rstest;

#[rstest]
fn show_single_commit_with_abbrev_commit_flag(
    init_repository_dir: TempDir,
) -> Result<(), Box<dyn std::error::Error>> {
    let repository_dir = init_repository_dir;

    // Get the commit SHA from the refs
    let expected_commit_sha = get_head_commit_sha(repository_dir.path())?;

    // Run the log command with --abbrev-commit flag
    let output = run_bit_command(repository_dir.path(), &["log", "--abbrev-commit"])
        .assert()
        .success();

    let stdout = String::from_utf8(output.get_output().stdout.clone())?;

    // With --abbrev-commit, the commit line should show abbreviated SHA
    // Format: commit <abbreviated-sha>
    assert!(
        stdout.starts_with("commit "),
        "Expected output to start with 'commit ', got:\n{}",
        stdout
    );

    // Extract and validate the abbreviated commit SHA
    let first_line = stdout.lines().next().unwrap();
    let after_commit_prefix = first_line.strip_prefix("commit ").unwrap();

    // Extract SHA (may be followed by decoration like " (HEAD -> master)")
    let displayed_commit_sha = if let Some(space_pos) = after_commit_prefix.find(' ') {
        &after_commit_prefix[..space_pos]
    } else {
        after_commit_prefix
    };

    // The abbreviated SHA should be 7 characters (default abbreviation length)
    assert_eq!(
        displayed_commit_sha.len(),
        7,
        "Expected 7-character abbreviated SHA, got {} characters: {}",
        displayed_commit_sha.len(),
        displayed_commit_sha
    );

    // Verify it's hexadecimal
    assert!(
        displayed_commit_sha.chars().all(|c| c.is_ascii_hexdigit()),
        "Expected hexadecimal SHA, got: {}",
        displayed_commit_sha
    );

    // Verify the abbreviated SHA matches the beginning of the full SHA
    assert!(
        expected_commit_sha.starts_with(displayed_commit_sha),
        "Abbreviated SHA '{}' does not match the beginning of full SHA '{}'",
        displayed_commit_sha,
        expected_commit_sha
    );

    // Check for Author, Date, and commit message (medium format)
    assert!(
        stdout.contains("Author: fake_user <fake_email@email.com>"),
        "Expected author in output:\n{}",
        stdout
    );

    assert!(
        stdout.contains("Date:"),
        "Expected date in output:\n{}",
        stdout
    );

    assert!(
        stdout.contains("    Initial commit"),
        "Expected indented commit message in output:\n{}",
        stdout
    );

    Ok(())
}
