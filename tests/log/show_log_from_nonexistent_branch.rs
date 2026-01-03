use crate::common::command::run_bit_command;
use assert_fs::TempDir;
use rstest::rstest;

#[rstest]
fn show_log_from_nonexistent_branch(
    #[from(crate::common::command::init_repository_dir)] repository_dir: TempDir,
) -> Result<(), Box<dyn std::error::Error>> {
    // Run the log command with a non-existent branch reference
    let output = run_bit_command(repository_dir.path(), &["log", "nonexistent-branch"])
        .assert()
        .success();

    let stdout = String::from_utf8(output.get_output().stdout.clone())?;

    assert!(
        stdout.is_empty() || stdout.contains("No commits"),
        "Expected empty output for log from non-existent branch, got: {}",
        stdout
    );

    Ok(())
}
