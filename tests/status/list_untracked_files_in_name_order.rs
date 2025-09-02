use crate::common::command::{repository_dir, run_bit_command};
use crate::common::file::write_generated_files;
use assert_fs::TempDir;
use pretty_assertions::assert_eq;
use rstest::rstest;

#[rstest]
fn list_untracked_files_in_name_order(
    repository_dir: TempDir,
) -> Result<(), Box<dyn std::error::Error>> {
    run_bit_command(repository_dir.path(), &["init"])
        .assert()
        .success();

    let mut files = write_generated_files(repository_dir.path(), 2);
    files.sort();

    let expected_output = files
        .iter()
        .map(|f| format!("?? {}", f.path.file_name().unwrap().to_string_lossy()))
        .collect::<Vec<_>>()
        .join("\n")
        + "\n";

    let actual_output = run_bit_command(repository_dir.path(), &["status"])
        .assert()
        .success();
    let stdout = actual_output.get_output().stdout.clone();
    let actual_output = String::from_utf8(stdout)?;

    assert_eq!(actual_output, expected_output);

    Ok(())
}
