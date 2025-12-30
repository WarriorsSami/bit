use crate::common::command::{get_head_commit_sha, init_repository_dir, run_bit_command};
use assert_fs::TempDir;
use rstest::rstest;

#[rstest]
fn show_single_commit_with_format_medium(
    init_repository_dir: TempDir,
) -> Result<(), Box<dyn std::error::Error>> {
    let repository_dir = init_repository_dir;

    // Get the commit SHA from the refs
    let expected_commit_sha = get_head_commit_sha(repository_dir.path())?;

    // Run the log command with --format=medium flag
    let output = run_bit_command(repository_dir.path(), &["log", "--format=medium"])
        .assert()
        .success();

    let stdout = String::from_utf8(output.get_output().stdout.clone())?;

    // Verify the output contains the commit in git's medium format
    // Medium format includes:
    // - commit <sha>
    // - Author: <name> <email>
    // - Date: <date>
    // - <blank line>
    // - <commit message indented by 4 spaces>

    // Check for commit line (should start with "commit " followed by a 40-char hex SHA)
    assert!(
        stdout.starts_with("commit "),
        "Expected output to start with 'commit ', got:\n{}",
        stdout
    );

    // Extract and validate the commit SHA format
    let first_line = stdout.lines().next().unwrap();
    let displayed_commit_sha = first_line.strip_prefix("commit ").unwrap();
    assert_eq!(
        displayed_commit_sha.len(),
        40,
        "Expected 40-character SHA in medium format, got {} characters: {}",
        displayed_commit_sha.len(),
        displayed_commit_sha
    );
    assert!(
        displayed_commit_sha.chars().all(|c| c.is_ascii_hexdigit()),
        "Expected hexadecimal SHA, got: {}",
        displayed_commit_sha
    );

    // Verify that the displayed SHA matches the one from refs
    assert_eq!(
        displayed_commit_sha, expected_commit_sha,
        "Displayed commit SHA does not match the SHA from refs.\nExpected: {}\nDisplayed: {}",
        expected_commit_sha, displayed_commit_sha
    );

    // Check for Author line
    assert!(
        stdout.contains("Author: fake_user <fake_email@email.com>"),
        "Expected author in output:\n{}",
        stdout
    );

    // Check for Date line
    assert!(
        stdout.contains("Date:"),
        "Expected date in output:\n{}",
        stdout
    );

    // Check for commit message (indented by 4 spaces)
    assert!(
        stdout.contains("    Initial commit"),
        "Expected indented commit message in output:\n{}",
        stdout
    );

    Ok(())
}
