use crate::common::command::{
    diff_hunks_output, file_b, init_repository_dir_for_diff_hunks, run_bit_command,
};
use crate::common::file::{FileSpec, write_file};
use assert_fs::TempDir;
use rstest::rstest;

#[rstest]
fn show_diff_with_hunks_for_multiple_files_with_modified_content_in_workspace(
    init_repository_dir_for_diff_hunks: TempDir,
    file_b: String,
    diff_hunks_output: String,
) -> Result<(), Box<dyn std::error::Error>> {
    let repository_dir = init_repository_dir_for_diff_hunks;

    // modify file 1.txt
    let file1_spec = FileSpec::new(repository_dir.path().join("1.txt"), file_b.clone());
    write_file(file1_spec);

    // modify file 2.txt
    let file2_spec = FileSpec::new(
        repository_dir.path().join("a").join("2.txt"),
        file_b.clone(),
    );
    write_file(file2_spec);

    let expected_output = format!(
        r#"diff --git a/1.txt b/1.txt
index 6143f6e..e0b1c3b 100644
--- a/1.txt
+++ b/1.txt
{}diff --git a/a/2.txt b/a/2.txt
index 6143f6e..e0b1c3b 100644
--- a/a/2.txt
+++ b/a/2.txt
{}"#,
        diff_hunks_output, diff_hunks_output
    );
    let actual_output = run_bit_command(repository_dir.path(), &["diff"])
        .assert()
        .success();
    let stdout = actual_output.get_output().stdout.clone();
    let actual_output = String::from_utf8(stdout)?;

    pretty_assertions::assert_eq!(actual_output, expected_output);

    Ok(())
}
