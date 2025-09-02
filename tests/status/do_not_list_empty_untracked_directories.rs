use crate::common::command::{repository_dir, run_bit_command};
use crate::common::file::create_directory;
use assert_fs::TempDir;
use pretty_assertions::assert_eq;
use rstest::rstest;

#[rstest]
fn do_not_list_empty_untracked_directories(
    repository_dir: TempDir,
) -> Result<(), Box<dyn std::error::Error>> {
    run_bit_command(repository_dir.path(), &["init"])
        .assert()
        .success();

    let outer_dir_path = repository_dir.path().join("outer");
    create_directory(&outer_dir_path);

    let expected_output = "".to_string();
    let actual_output = run_bit_command(repository_dir.path(), &["status"])
        .assert()
        .success();
    let stdout = actual_output.get_output().stdout.clone();
    let actual_output = String::from_utf8(stdout)?;

    assert_eq!(actual_output, expected_output);

    Ok(())
}
