use crate::common::command::{
    bit_commit, get_head_commit_sha, get_parent_commit_id, run_bit_command,
};
use crate::common::file::{FileSpec, write_file};
use assert_fs::TempDir;
use rstest::rstest;

#[rstest]
fn show_linear_history_in_medium_format(
    #[from(crate::common::command::repository_dir)] repository_dir: TempDir,
) -> Result<(), Box<dyn std::error::Error>> {
    // Initialize repository
    run_bit_command(repository_dir.path(), &["init"])
        .assert()
        .success();

    // Create a linear history with 5 commits
    let commit_messages = [
        "First commit - initialize project",
        "Second commit - add feature A",
        "Third commit - fix bug in feature A",
        "Fourth commit - add feature B",
        "Fifth commit - refactor code",
    ];

    for (i, message) in commit_messages.iter().enumerate() {
        let file = FileSpec::new(
            repository_dir.path().join(format!("file{}.txt", i + 1)),
            format!("Content for file {}", i + 1),
        );
        write_file(file);
        run_bit_command(repository_dir.path(), &["add", "."])
            .assert()
            .success();
        bit_commit(repository_dir.path(), message)
            .assert()
            .success();
    }

    // Get commit SHAs from refs (in reverse chronological order)
    let expected_commit5_sha = get_head_commit_sha(repository_dir.path())?;
    let expected_commit4_sha = get_parent_commit_id(repository_dir.path(), &expected_commit5_sha)?;
    let expected_commit3_sha = get_parent_commit_id(repository_dir.path(), &expected_commit4_sha)?;
    let expected_commit2_sha = get_parent_commit_id(repository_dir.path(), &expected_commit3_sha)?;
    let expected_commit1_sha = get_parent_commit_id(repository_dir.path(), &expected_commit2_sha)?;

    let expected_shas = [
        expected_commit5_sha.as_str(),
        expected_commit4_sha.as_str(),
        expected_commit3_sha.as_str(),
        expected_commit2_sha.as_str(),
        expected_commit1_sha.as_str(),
    ];

    // Run the log command
    let output = run_bit_command(repository_dir.path(), &["log", "--decorate=none"])
        .assert()
        .success();

    let stdout = String::from_utf8(output.get_output().stdout.clone())?;

    // Extract all commit SHAs from the output
    let commit_shas: Vec<&str> = stdout
        .lines()
        .filter(|line| line.starts_with("commit "))
        .map(|line| line.strip_prefix("commit ").unwrap())
        .collect();

    // Verify we have exactly 5 commits
    assert_eq!(
        commit_shas.len(),
        5,
        "Expected 5 commits in output, found {}",
        commit_shas.len()
    );

    // Verify all SHAs are valid 40-character hex strings
    for sha in &commit_shas {
        assert_eq!(sha.len(), 40, "Expected 40-character SHA, got: {}", sha);
        assert!(
            sha.chars().all(|c| c.is_ascii_hexdigit()),
            "Expected hexadecimal SHA, got: {}",
            sha
        );
    }

    // Verify that the displayed SHAs match the expected SHAs from refs
    for (i, (displayed, expected)) in commit_shas.iter().zip(expected_shas.iter()).enumerate() {
        assert_eq!(
            displayed,
            expected,
            "Commit {} SHA mismatch.\nExpected: {}\nDisplayed: {}",
            5 - i,
            expected,
            displayed
        );
    }

    // Verify all commits are present in the output
    for (i, sha) in commit_shas.iter().enumerate() {
        assert!(
            stdout.contains(&format!("commit {}", sha)),
            "Expected commit {} (SHA: {}) in output:\n{}",
            5 - i,
            sha,
            stdout
        );
    }

    // Verify all commit messages are present and properly indented
    for message in commit_messages.iter() {
        assert!(
            stdout.contains(&format!("    {}", message)),
            "Expected commit message '{}' indented by 4 spaces in output:\n{}",
            message,
            stdout
        );
    }

    // Verify commits are in reverse chronological order (newest first)
    let mut positions = Vec::new();
    for sha in commit_shas.iter() {
        let pos = stdout
            .find(&format!("commit {}", sha))
            .unwrap_or_else(|| panic!("Could not find commit {}", sha));
        positions.push(pos);
    }

    for i in 0..positions.len() - 1 {
        assert!(
            positions[i] < positions[i + 1],
            "Expected commit {} to appear before commit {}",
            5 - i,
            5 - i - 1
        );
    }

    // Verify the format structure for each commit
    // Each commit should have: commit line, Author line, Date line, blank line, indented message

    // Reverse the commit messages to match the reverse chronological order of commit_shas
    let commit_messages_reversed: Vec<&str> = commit_messages.iter().rev().copied().collect();

    // Verify structure for each commit by searching for it in the output
    for (i, (sha, message)) in commit_shas
        .iter()
        .zip(commit_messages_reversed.iter())
        .enumerate()
    {
        // Find the position of this commit in the output
        let commit_line = format!("commit {}", sha);
        let commit_pos = stdout
            .find(&commit_line)
            .unwrap_or_else(|| panic!("Could not find commit line for SHA {}", sha));

        // Extract the section starting from this commit
        let commit_section = &stdout[commit_pos..];

        // Find where the next commit starts (if any) to isolate this commit's block
        let next_commit_pos = commit_section[1..].find("\ncommit ").map(|pos| pos + 1);

        let commit_block = if let Some(end) = next_commit_pos {
            &commit_section[..end]
        } else {
            commit_section
        };

        let lines: Vec<&str> = commit_block.lines().collect();

        // Line 1: commit <sha>
        assert!(
            lines[0].starts_with("commit "),
            "Commit {} ({}): First line should start with 'commit ', got: {}",
            i + 1,
            sha,
            lines[0]
        );
        assert!(
            lines[0].contains(sha),
            "Commit {} ({}): First line should contain commit SHA, got: {}",
            i + 1,
            sha,
            lines[0]
        );

        // Line 2: Author: <name> <email>
        assert!(
            lines.len() > 1 && lines[1].starts_with("Author: "),
            "Commit {} ({}): Second line should start with 'Author: ', got: {:?}",
            i + 1,
            sha,
            lines.get(1)
        );
        assert!(
            lines[1].contains("fake_user") && lines[1].contains("fake_email@email.com"),
            "Commit {} ({}): Author line should contain name and email, got: {}",
            i + 1,
            sha,
            lines[1]
        );

        // Line 3: Date: <date>
        assert!(
            lines.len() > 2 && lines[2].starts_with("Date: "),
            "Commit {} ({}): Third line should start with 'Date: ', got: {:?}",
            i + 1,
            sha,
            lines.get(2)
        );

        // Line 4: Empty line
        assert!(
            lines.len() > 3 && (lines[3].is_empty() || lines[3].trim().is_empty()),
            "Commit {} ({}): Fourth line should be empty, got: '{}'",
            i + 1,
            sha,
            lines.get(3).unwrap_or(&"<missing>")
        );

        // Line 5+: Commit message indented by 4 spaces
        assert!(
            lines.len() > 4 && lines[4].starts_with("    "),
            "Commit {} ({}): Commit message should be indented by 4 spaces, got: '{}'",
            i + 1,
            sha,
            lines.get(4).unwrap_or(&"<missing>")
        );
        assert!(
            lines[4].contains(message),
            "Commit {} ({}): Expected message '{}' in line: '{}'",
            i + 1,
            sha,
            message,
            lines[4]
        );
    }

    // Verify total count of Author and Date lines matches number of commits
    let author_count = stdout.matches("Author:").count();
    assert_eq!(
        author_count, 5,
        "Expected 5 author lines, found {}",
        author_count
    );

    let date_count = stdout.matches("Date:").count();
    assert_eq!(date_count, 5, "Expected 5 date lines, found {}", date_count);

    // Verify that the log starts with the most recent commit (first in the list)
    assert!(
        stdout.starts_with(&format!("commit {}", commit_shas[0])),
        "Expected log to start with the most recent commit:\n{}",
        stdout
    );

    Ok(())
}
