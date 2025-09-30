use crate::common::command::{init_repository_dir, run_bit_command};
use crate::common::file::{FileSpec, write_file};
use assert_fs::TempDir;
use rstest::rstest;

#[rstest]
fn report_a_file_added_to_an_untracked_directory(
    init_repository_dir: TempDir,
) -> Result<(), Box<dyn std::error::Error>> {
    let repository_dir = init_repository_dir;

    let file = FileSpec::new(
        repository_dir.path().join("d").join("e").join("5.txt"),
        "five".to_string(),
    );
    write_file(file);

    run_bit_command(repository_dir.path(), &["add", "."])
        .assert()
        .success();

    let expected_output = "A  d/e/5.txt\n".to_string();
    let actual_output = run_bit_command(repository_dir.path(), &["status"])
        .assert()
        .success();
    let stdout = actual_output.get_output().stdout.clone();
    let actual_output = String::from_utf8(stdout)?;

    pretty_assertions::assert_eq!(actual_output, expected_output);

    Ok(())
}
