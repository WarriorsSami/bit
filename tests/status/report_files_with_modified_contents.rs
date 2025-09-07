use crate::common::command::init_repository_dir;
use crate::common::file::{FileSpec, write_file};
use assert_fs::TempDir;
use pretty_assertions::assert_eq;
use rstest::rstest;

#[rstest]
fn report_files_with_modified_contents(
    init_repository_dir: TempDir,
) -> Result<(), Box<dyn std::error::Error>> {
    let repository_dir = init_repository_dir;

    let file1 = FileSpec::new(
        repository_dir.path().join("1.txt"),
        "modified one".to_string(),
    );
    write_file(file1);

    let file2 = FileSpec::new(
        repository_dir.path().join("a").join("2.txt"),
        "modified two".to_string(),
    );
    write_file(file2);

    let expected_output = " M 1.txt\n M a/2.txt\n".to_string();
    let actual_output = crate::common::command::run_bit_command(repository_dir.path(), &["status"])
        .assert()
        .success();
    let stdout = actual_output.get_output().stdout.clone();
    let actual_output = String::from_utf8(stdout)?;

    assert_eq!(actual_output, expected_output);

    Ok(())
}
