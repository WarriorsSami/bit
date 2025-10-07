use crate::common::command::init_repository_dir;
use crate::common::file::touch_file;
use assert_fs::TempDir;
use pretty_assertions::assert_eq;
use rstest::rstest;

#[rstest]
fn print_nothing_if_a_file_is_touched(
    init_repository_dir: TempDir,
) -> Result<(), Box<dyn std::error::Error>> {
    let repository_dir = init_repository_dir;

    let file1 = repository_dir.path().join("1.txt");
    touch_file(&file1);

    let expected_output = "".to_string();
    let actual_output =
        crate::common::command::run_bit_command(repository_dir.path(), &["status", "--porcelain"])
            .assert()
            .success();
    let stdout = actual_output.get_output().stdout.clone();
    let actual_output = String::from_utf8(stdout)?;

    assert_eq!(actual_output, expected_output);

    Ok(())
}
