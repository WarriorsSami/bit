use crate::common::command::{
    bit_commit, get_head_commit_sha, get_parent_commit_id, init_repository_dir, run_bit_command,
};
use crate::common::file::{FileSpec, write_file};
use assert_fs::TempDir;
use rstest::rstest;

#[rstest]
fn show_multiple_commits_with_abbrev_commit_flag(
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

    // Run the log command with --abbrev-commit flag
    let output = run_bit_command(repository_dir.path(), &["log", "--abbrev-commit"])
        .assert()
        .success();

    let stdout = String::from_utf8(output.get_output().stdout.clone())?;

    // Extract all commit SHAs from the log output
    let displayed_shas: Vec<&str> = stdout
        .lines()
        .filter(|line| line.starts_with("commit "))
        .map(|line| {
            let after_prefix = line.strip_prefix("commit ").unwrap();
            // Extract SHA (may be followed by decoration)
            if let Some(space_pos) = after_prefix.find(' ') {
                &after_prefix[..space_pos]
            } else {
                after_prefix
            }
        })
        .collect();

    // Verify we have exactly 3 commits
    assert_eq!(
        displayed_shas.len(),
        3,
        "Expected 3 commits in output, found {}:\n{}",
        displayed_shas.len(),
        stdout
    );

    // Verify all SHAs are abbreviated (7 characters) and match the expected full SHAs
    for (i, (displayed, expected)) in displayed_shas.iter().zip(expected_shas.iter()).enumerate() {
        assert_eq!(
            displayed.len(),
            7,
            "Expected 7-character abbreviated SHA for commit {}, got: {}",
            i + 1,
            displayed
        );
        assert!(
            displayed.chars().all(|c| c.is_ascii_hexdigit()),
            "Expected hexadecimal SHA for commit {}, got: {}",
            i + 1,
            displayed
        );

        // Verify the abbreviated SHA matches the beginning of the full SHA
        assert!(
            expected.starts_with(displayed),
            "Abbreviated SHA '{}' does not match the beginning of full SHA '{}'",
            displayed,
            expected
        );
    }

    // Verify commit messages are present and indented (medium format)
    assert!(
        stdout.contains("    Third commit"),
        "Expected third commit message in output:\n{}",
        stdout
    );

    assert!(
        stdout.contains("    Second commit"),
        "Expected second commit message in output:\n{}",
        stdout
    );

    assert!(
        stdout.contains("    Initial commit"),
        "Expected initial commit message in output:\n{}",
        stdout
    );

    // Verify Author and Date lines are present for each commit (medium format)
    let author_count = stdout
        .matches("Author: fake_user <fake_email@email.com>")
        .count();
    assert_eq!(
        author_count, 3,
        "Expected 3 author lines, found {}:\n{}",
        author_count, stdout
    );

    let date_count = stdout.matches("Date:").count();
    assert_eq!(
        date_count, 3,
        "Expected 3 date lines, found {}:\n{}",
        date_count, stdout
    );

    Ok(())
}
