use crate::common::command::{bit_commit, repository_dir, run_bit_command};
use crate::common::file::write_generated_files;
use assert_fs::TempDir;
use pretty_assertions::assert_eq;
use rstest::rstest;

#[rstest]
fn list_files_as_untracked_if_they_are_not_in_the_index(
    repository_dir: TempDir,
) -> Result<(), Box<dyn std::error::Error>> {
    run_bit_command(repository_dir.path(), &["init"])
        .assert()
        .success();

    let files = write_generated_files(repository_dir.path(), 1);
    let committed_file = files.first().unwrap();

    run_bit_command(
        repository_dir.path(),
        &["add", committed_file.path.to_str().unwrap()],
    )
    .assert()
    .success();

    bit_commit(repository_dir.path(), "Initial commit")
        .assert()
        .success();

    let files = write_generated_files(repository_dir.path(), 1);
    let untracked_file = files.first().unwrap();

    let expected_output = format!(
        "?? {}\n",
        untracked_file.path.file_name().unwrap().to_string_lossy()
    );

    let actual_output = run_bit_command(repository_dir.path(), &["status", "--porcelain"])
        .assert()
        .success();
    let stdout = actual_output.get_output().stdout.clone();
    let actual_output = String::from_utf8(stdout)?;

    assert_eq!(actual_output, expected_output);

    Ok(())
}
