use crate::common::command::{get_head_commit_sha, init_repository_dir, run_bit_command};
use assert_fs::TempDir;
use rstest::rstest;

#[rstest]
fn show_single_commit_with_oneline_and_abbrev_commit_flags(
    init_repository_dir: TempDir,
) -> Result<(), Box<dyn std::error::Error>> {
    let repository_dir = init_repository_dir;

    // Get the commit SHA from the refs
    let expected_commit_sha = get_head_commit_sha(repository_dir.path())?;

    // Run the log command with both --oneline and --abbrev-commit flags
    // (--abbrev-commit is implicit with --oneline, but testing both together)
    let output = run_bit_command(
        repository_dir.path(),
        &["log", "--oneline", "--abbrev-commit"],
    )
    .assert()
    .success();

    let stdout = String::from_utf8(output.get_output().stdout.clone())?;

    // In oneline format, each commit should be on a single line
    // Format: <abbreviated-sha> <commit-message-first-line>
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
    // Format can be: "abc1234 message" or "abc1234 (HEAD -> master) message"
    let sha_end = first_line.find(' ').unwrap();
    let displayed_abbreviated_sha = &first_line[..sha_end];

    // Verify the abbreviated SHA is 7 characters
    assert_eq!(
        displayed_abbreviated_sha.len(),
        7,
        "Expected 7-character abbreviated SHA, got {} characters: {}",
        displayed_abbreviated_sha.len(),
        displayed_abbreviated_sha
    );

    // Verify it's hexadecimal
    assert!(
        displayed_abbreviated_sha
            .chars()
            .all(|c| c.is_ascii_hexdigit()),
        "Expected hexadecimal SHA, got: {}",
        displayed_abbreviated_sha
    );

    // Verify the abbreviated SHA matches the beginning of the full SHA
    assert!(
        expected_commit_sha.starts_with(displayed_abbreviated_sha),
        "Abbreviated SHA '{}' does not match the beginning of full SHA '{}'",
        displayed_abbreviated_sha,
        expected_commit_sha
    );

    // Extract the commit message (skip SHA and potential decoration)
    let rest = &first_line[sha_end..].trim_start();
    let commit_message = if rest.starts_with('(') {
        // Has decoration, skip it
        if let Some(closing_paren) = rest.find(')') {
            rest[closing_paren + 1..].trim()
        } else {
            rest
        }
    } else {
        rest
    };

    // Verify the commit message
    assert_eq!(
        commit_message, "Initial commit",
        "Expected commit message 'Initial commit', got: {}",
        commit_message
    );

    Ok(())
}
