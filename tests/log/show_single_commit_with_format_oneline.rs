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

    // The line should start with a SHA (40 chars in oneline format when using --format)
    // Format can be: "abc...xyz message" or "abc...xyz (HEAD -> master) message"
    let sha_end = first_line.find(' ').unwrap();
    let displayed_sha = &first_line[..sha_end];

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

    // Verify the SHA matches the expected SHA
    assert_eq!(
        displayed_sha, expected_commit_sha,
        "SHA '{}' does not match expected SHA '{}'",
        displayed_sha, expected_commit_sha
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
