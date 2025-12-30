use crate::common::command::{
    bit_commit, get_head_commit_sha, get_parent_commit_id, init_repository_dir, run_bit_command,
};
use crate::common::file::{FileSpec, write_file};
use assert_fs::TempDir;
use rstest::rstest;

#[rstest]
fn show_multiple_commits_in_oneline_format(
    init_repository_dir: TempDir,
) -> Result<(), Box<dyn std::error::Error>> {
    let repository_dir = init_repository_dir;

    // Create second commit
    let file2 = FileSpec::new(
        repository_dir.path().join("file2.txt"),
        "second file content".to_string(),
    );
    write_file(file2);
    run_bit_command(repository_dir.path(), &["add", "file2.txt"])
        .assert()
        .success();
    bit_commit(repository_dir.path(), "Second commit")
        .assert()
        .success();

    // Create third commit
    let file3 = FileSpec::new(
        repository_dir.path().join("file3.txt"),
        "third file content".to_string(),
    );
    write_file(file3);
    run_bit_command(repository_dir.path(), &["add", "file3.txt"])
        .assert()
        .success();
    bit_commit(repository_dir.path(), "Third commit")
        .assert()
        .success();

    // Get commit SHAs from refs
    let expected_commit3_sha = get_head_commit_sha(repository_dir.path())?;
    let expected_commit2_sha = get_parent_commit_id(repository_dir.path(), &expected_commit3_sha)?;
    let expected_commit1_sha = get_parent_commit_id(repository_dir.path(), &expected_commit2_sha)?;

    let expected_shas = [
        expected_commit3_sha.as_str(),
        expected_commit2_sha.as_str(),
        expected_commit1_sha.as_str(),
    ];

    // Run the log command with --oneline flag
    let output = run_bit_command(repository_dir.path(), &["log", "--oneline"])
        .assert()
        .success();

    let stdout = String::from_utf8(output.get_output().stdout.clone())?;

    // In oneline format, each commit should be on a single line
    let lines: Vec<&str> = stdout
        .trim()
        .lines()
        .filter(|line| !line.is_empty())
        .collect();

    // Verify we have exactly 3 commits
    assert_eq!(
        lines.len(),
        3,
        "Expected 3 commit lines in oneline format, got {}:\n{}",
        lines.len(),
        stdout
    );

    // Parse each line and verify format
    let expected_messages = ["Third commit", "Second commit", "Initial commit"];

    for (i, (line, (expected_sha, expected_message))) in lines
        .iter()
        .zip(expected_shas.iter().zip(expected_messages.iter()))
        .enumerate()
    {
        let parts: Vec<&str> = line.splitn(2, ' ').collect();
        assert_eq!(
            parts.len(),
            2,
            "Line {} should have SHA and message separated by space, got: {}",
            i + 1,
            line
        );

        let displayed_abbreviated_sha = parts[0];
        let commit_message = parts[1];

        // Verify the abbreviated SHA is 7 characters
        assert_eq!(
            displayed_abbreviated_sha.len(),
            7,
            "Expected 7-character abbreviated SHA for commit {}, got {} characters: {}",
            i + 1,
            displayed_abbreviated_sha.len(),
            displayed_abbreviated_sha
        );

        // Verify it's hexadecimal
        assert!(
            displayed_abbreviated_sha
                .chars()
                .all(|c| c.is_ascii_hexdigit()),
            "Expected hexadecimal SHA for commit {}, got: {}",
            i + 1,
            displayed_abbreviated_sha
        );

        // Verify the abbreviated SHA matches the beginning of the full SHA
        assert!(
            expected_sha.starts_with(displayed_abbreviated_sha),
            "Abbreviated SHA '{}' does not match the beginning of full SHA '{}'",
            displayed_abbreviated_sha,
            expected_sha
        );

        // Verify the commit message
        assert_eq!(
            commit_message, *expected_message,
            "Expected commit message '{}', got: {}",
            expected_message, commit_message
        );
    }

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
