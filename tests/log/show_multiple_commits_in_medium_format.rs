use crate::common::command::{
    bit_commit, get_head_commit_sha, get_parent_commit_id, init_repository_dir, run_bit_command,
};
use crate::common::file::{FileSpec, write_file};
use assert_fs::TempDir;
use rstest::rstest;

#[rstest]
fn show_multiple_commits_in_medium_format(
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

    // Run the log command
    let output = run_bit_command(repository_dir.path(), &["log"])
        .assert()
        .success();

    let stdout = String::from_utf8(output.get_output().stdout.clone())?;

    // Extract all commit SHAs from the log output
    let displayed_shas: Vec<&str> = stdout
        .lines()
        .filter(|line| line.starts_with("commit "))
        .map(|line| line.strip_prefix("commit ").unwrap())
        .collect();

    // Verify we have exactly 3 commits
    assert_eq!(
        displayed_shas.len(),
        3,
        "Expected 3 commits in output, found {}:\n{}",
        displayed_shas.len(),
        stdout
    );

    // Verify all SHAs are valid 40-character hex strings
    for sha in &displayed_shas {
        assert_eq!(sha.len(), 40, "Expected 40-character SHA, got: {}", sha);
        assert!(
            sha.chars().all(|c| c.is_ascii_hexdigit()),
            "Expected hexadecimal SHA, got: {}",
            sha
        );
    }

    // Verify that the displayed SHAs match the expected SHAs from refs
    for (i, (displayed, expected)) in displayed_shas.iter().zip(expected_shas.iter()).enumerate() {
        assert_eq!(
            displayed,
            expected,
            "Commit {} SHA mismatch.\nExpected: {}\nDisplayed: {}",
            i + 1,
            expected,
            displayed
        );
    }

    // Verify commit messages are present and indented
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

    // Verify the order: Third commit should appear before Second commit,
    // and Second commit before Initial commit (reverse chronological order)
    let third_pos = stdout.find("    Third commit").unwrap();
    let second_pos = stdout.find("    Second commit").unwrap();
    let initial_pos = stdout.find("    Initial commit").unwrap();

    assert!(
        third_pos < second_pos,
        "Expected 'Third commit' to appear before 'Second commit'"
    );
    assert!(
        second_pos < initial_pos,
        "Expected 'Second commit' to appear before 'Initial commit'"
    );

    // Verify Author and Date lines are present for each commit
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
