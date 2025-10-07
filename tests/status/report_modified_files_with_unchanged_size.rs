use crate::common::command::init_repository_dir;
use crate::common::file::{FileSpec, write_file};
use assert_fs::TempDir;
use pretty_assertions::assert_eq;
use rstest::rstest;

#[rstest]
fn report_modified_files_with_unchanged_size(
    init_repository_dir: TempDir,
) -> Result<(), Box<dyn std::error::Error>> {
    let repository_dir = init_repository_dir;

    let file3 = FileSpec::new(
        repository_dir.path().join("a").join("b").join("3.txt"),
        "hello".to_string(),
    );
    write_file(file3);

    let expected_output = " M a/b/3.txt\n".to_string();
    let actual_output =
        crate::common::command::run_bit_command(repository_dir.path(), &["status", "--porcelain"])
            .assert()
            .success();
    let stdout = actual_output.get_output().stdout.clone();
    let actual_output = String::from_utf8(stdout)?;

    assert_eq!(actual_output, expected_output);

    Ok(())
}
