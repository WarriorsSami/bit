use crate::common::command::{init_repository_dir, run_bit_command};
use crate::common::file::delete_path;
use assert_fs::TempDir;
use rstest::rstest;

#[rstest]
fn report_deleted_files_from_last_commit(
    init_repository_dir: TempDir,
) -> Result<(), Box<dyn std::error::Error>> {
    let repository_dir = init_repository_dir;

    let file1 = repository_dir.path().join("1.txt");
    delete_path(&file1);

    let git_index_path = repository_dir.path().join(".git").join("index");
    delete_path(&git_index_path);

    run_bit_command(repository_dir.path(), &["add", "."])
        .assert()
        .success();

    let expected_output = "D  1.txt\n".to_string();
    let actual_output = run_bit_command(repository_dir.path(), &["status", "--porcelain"])
        .assert()
        .success();
    let stdout = actual_output.get_output().stdout.clone();
    let actual_output = String::from_utf8(stdout)?;

    pretty_assertions::assert_eq!(actual_output, expected_output);

    Ok(())
}
