use crate::common::command::{init_repository_dir, run_bit_command};
use crate::common::file::delete_path;
use assert_fs::TempDir;
use rstest::rstest;

#[rstest]
fn report_all_deleted_files_inside_directories_from_last_commit(
    init_repository_dir: TempDir,
) -> Result<(), Box<dyn std::error::Error>> {
    let repository_dir = init_repository_dir;

    let dir = repository_dir.path().join("a");
    delete_path(&dir);

    let git_index_path = repository_dir.path().join(".git").join("index");
    delete_path(&git_index_path);

    run_bit_command(repository_dir.path(), &["add", "."])
        .assert()
        .success();

    let expected_output = "D  a/2.txt\nD  a/b/3.txt\n".to_string();
    let actual_output = run_bit_command(repository_dir.path(), &["status"])
        .assert()
        .success();
    let stdout = actual_output.get_output().stdout.clone();
    let actual_output = String::from_utf8(stdout)?;

    pretty_assertions::assert_eq!(actual_output, expected_output);

    Ok(())
}
