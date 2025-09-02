use crate::common::command::{repository_dir, run_bit_command};
use crate::common::file::{FileSpec, write_file};
use assert_fs::TempDir;
use pretty_assertions::assert_eq;
use rstest::rstest;

#[rstest]
fn list_untracked_directories_that_indirectly_contain_files(
    repository_dir: TempDir,
) -> Result<(), Box<dyn std::error::Error>> {
    run_bit_command(repository_dir.path(), &["init"])
        .assert()
        .success();

    let file = FileSpec::new(
        repository_dir
            .path()
            .join("outer")
            .join("inner")
            .join("file.txt"),
        String::new(),
    );
    write_file(file);

    let expected_output = "?? outer/\n".to_string();
    let actual_output = run_bit_command(repository_dir.path(), &["status"])
        .assert()
        .success();
    let stdout = actual_output.get_output().stdout.clone();
    let actual_output = String::from_utf8(stdout)?;

    assert_eq!(actual_output, expected_output);

    Ok(())
}
