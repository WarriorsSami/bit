use crate::common::command::{bit_commit, get_head_commit_sha, run_bit_command};
use crate::common::file::{FileSpec, write_file};
use assert_fs::TempDir;
use rstest::rstest;

#[rstest]
fn verify_medium_format_structure(
    #[from(crate::common::command::repository_dir)] repository_dir: TempDir,
) -> Result<(), Box<dyn std::error::Error>> {
    // Initialize repository
    run_bit_command(repository_dir.path(), &["init"])
        .assert()
        .success();

    // Create a commit with a multi-line message
    let file = FileSpec::new(
        repository_dir.path().join("test.txt"),
        "test content".to_string(),
    );
    write_file(file);
    run_bit_command(repository_dir.path(), &["add", "."])
        .assert()
        .success();

    let commit_message =
        "Short summary line\n\nDetailed description of changes.\nSecond line of details.";
    bit_commit(repository_dir.path(), commit_message)
        .assert()
        .success();

    // Get the expected commit SHA from refs
    let expected_commit_sha = get_head_commit_sha(repository_dir.path())?;

    // Run the log command
    let output = run_bit_command(repository_dir.path(), &["log"])
        .assert()
        .success();

    let stdout = String::from_utf8(output.get_output().stdout.clone())?;

    // Parse the output to verify the exact structure
    let lines: Vec<&str> = stdout.lines().collect();

    // Line 1: commit <sha>
    assert!(
        lines[0].starts_with("commit "),
        "First line should start with 'commit ', got: {}",
        lines[0]
    );

    // Extract and validate the commit SHA
    let after_commit_prefix = lines[0].strip_prefix("commit ").unwrap();

    // Extract SHA (may be followed by decoration like " (HEAD -> master)")
    let displayed_commit_sha = if let Some(space_pos) = after_commit_prefix.find(' ') {
        &after_commit_prefix[..space_pos]
    } else {
        after_commit_prefix
    };

    assert_eq!(
        displayed_commit_sha.len(),
        40,
        "Expected 40-character SHA, got {} characters: {}",
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
    // Line 2: Author: <name> <email>
    assert!(
        lines[1].starts_with("Author: "),
        "Second line should start with 'Author: ', got: {}",
        lines[1]
    );
    assert!(
        lines[1].contains("fake_user"),
        "Author line should contain author name, got: {}",
        lines[1]
    );
    assert!(
        lines[1].contains("fake_email@email.com"),
        "Author line should contain author email, got: {}",
        lines[1]
    );

    // Line 3: Date: <date>
    assert!(
        lines[2].starts_with("Date: "),
        "Third line should start with 'Date: ', got: {}",
        lines[2]
    );
    // Date format should be like: "Sun Jan 1 12:00:00 2023 +0000" (git's default format)
    // or similar readable format

    // Line 4: Empty line
    assert!(
        lines[3].is_empty() || lines[3].trim().is_empty(),
        "Fourth line should be empty, got: '{}'",
        lines[3]
    );

    // Lines 5+: Commit message indented by 4 spaces
    assert!(
        lines[4].starts_with("    "),
        "Commit message should be indented by 4 spaces, got: '{}'",
        lines[4]
    );
    assert!(
        lines[4].contains("Short summary line"),
        "Commit message should contain the summary, got: '{}'",
        lines[4]
    );

    // Verify multi-line message is properly formatted
    if commit_message.contains('\n') {
        // Check that subsequent message lines are also indented
        for line in &lines[4..] {
            if !line.is_empty() {
                assert!(
                    line.starts_with("    "),
                    "All commit message lines should be indented by 4 spaces, got: '{}'",
                    line
                );
            }
        }
    }

    Ok(())
}
