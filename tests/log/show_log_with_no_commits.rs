use crate::common::command::run_bit_command;
use assert_fs::TempDir;
use rstest::rstest;

#[rstest]
fn show_log_with_no_commits(
    #[from(crate::common::command::repository_dir)] repository_dir: TempDir,
) -> Result<(), Box<dyn std::error::Error>> {
    // Initialize repository but don't create any commits
    run_bit_command(repository_dir.path(), &["init"])
        .assert()
        .success();

    // Run the log command on an empty repository
    let output = run_bit_command(repository_dir.path(), &["log"])
        .assert()
        .success();

    let stdout = String::from_utf8(output.get_output().stdout.clone())?;

    // With no commits, the output should be empty or contain an appropriate message
    // This depends on implementation - git shows nothing for empty repos
    // For now, we just verify it doesn't crash and produces some output
    // (empty string is acceptable)
    assert!(
        stdout.is_empty()
            || stdout.trim().is_empty()
            || stdout.contains("fatal")
            || stdout.contains("No commits"),
        "Expected empty output or error message for repository with no commits, got: {}",
        stdout
    );

    Ok(())
}
