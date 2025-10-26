use crate::common::command::{init_repository_dir, run_bit_command};
use crate::common::file::delete_path;
use assert_fs::TempDir;
use rstest::rstest;

#[rstest]
fn show_diff_for_file_deleted_from_workspace(
    init_repository_dir: TempDir,
) -> Result<(), Box<dyn std::error::Error>> {
    let repository_dir = init_repository_dir;

    // delete file 1.txt
    let file1 = repository_dir.path().join("1.txt");
    delete_path(&file1);

    let expected_output =
        "diff --git a/1.txt b/1.txt\ndeleted file mode 100644\nindex 43dd47e..0000000\n--- a/1.txt\n+++ /dev/null\n"
            .to_string();
    let actual_output = run_bit_command(repository_dir.path(), &["diff"])
        .assert()
        .success();
    let stdout = actual_output.get_output().stdout.clone();
    let actual_output = String::from_utf8(stdout)?;

    pretty_assertions::assert_eq!(actual_output, expected_output);

    Ok(())
}
