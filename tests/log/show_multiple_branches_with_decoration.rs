use crate::common::command::{bit_commit, init_repository_dir, run_bit_command};
use crate::common::file::{FileSpec, write_file};
use assert_fs::TempDir;
use rstest::rstest;

#[rstest]
fn show_multiple_branches_with_decoration(
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

    // Create a new branch at the second commit (HEAD~1)
    run_bit_command(
        repository_dir.path(),
        &["branch", "create", "feature-branch", "HEAD~1"],
    )
    .assert()
    .success();

    // Run the log command with --decorate=short
    let output = run_bit_command(repository_dir.path(), &["log", "--decorate", "short"])
        .assert()
        .success();

    let stdout = String::from_utf8(output.get_output().stdout.clone())?;
    let lines: Vec<&str> = stdout.lines().collect();

    // First commit (HEAD) should have decoration: (HEAD -> master)
    assert!(
        lines[0].contains("(HEAD -> master)") || lines[0].contains("(HEAD -> main)"),
        "Expected first commit to have '(HEAD -> master)' or '(HEAD -> main)', but got: {}",
        lines[0]
    );

    // Find the commit line for the second commit (should have feature-branch decoration)
    let second_commit_line = lines
        .iter()
        .find(|line| line.starts_with("commit ") && line.contains("feature-branch"))
        .expect("Expected to find a commit with feature-branch decoration");

    assert!(
        second_commit_line.contains("(feature-branch)"),
        "Expected second commit to have '(feature-branch)' decoration, but got: {}",
        second_commit_line
    );

    Ok(())
}
