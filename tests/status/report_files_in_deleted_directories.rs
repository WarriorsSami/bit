use crate::common::command::init_repository_dir;
use crate::common::file::delete_path;
use assert_fs::TempDir;
use pretty_assertions::assert_eq;
use rstest::rstest;

#[rstest]
fn report_files_in_deleted_directories(
    init_repository_dir: TempDir,
) -> Result<(), Box<dyn std::error::Error>> {
    let repository_dir = init_repository_dir;

    let dir = repository_dir.path().join("a");
    delete_path(&dir);

    let expected_output = " D a/2.txt\n D a/b/3.txt\n".to_string();
    let actual_output = crate::common::command::run_bit_command(repository_dir.path(), &["status"])
        .assert()
        .success();
    let stdout = actual_output.get_output().stdout.clone();
    let actual_output = String::from_utf8(stdout)?;

    assert_eq!(actual_output, expected_output);

    Ok(())
}
