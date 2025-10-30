use crate::common::command::{init_repository_dir, run_bit_command};
use crate::common::file::{FileSpec, write_file};
use assert_fs::TempDir;
use rstest::rstest;

#[rstest]
fn show_cached_diff_for_file_with_modified_content_in_index(
    init_repository_dir: TempDir,
) -> Result<(), Box<dyn std::error::Error>> {
    let repository_dir = init_repository_dir;

    // modify file 1.txt
    let file1_spec = FileSpec::new(
        repository_dir.path().join("1.txt"),
        "modified one".to_string(),
    );
    write_file(file1_spec);

    // add the modified file to the index
    run_bit_command(repository_dir.path(), &["add", "1.txt"])
        .assert()
        .success();

    let expected_output =
        "diff --git a/1.txt b/1.txt\nindex 43dd47e..ba9bbba 100644\n--- a/1.txt\n+++ b/1.txt\n-one\n+modified one\n"
            .to_string();
    let actual_output = run_bit_command(repository_dir.path(), &["diff", "--cached"])
        .assert()
        .success();
    let stdout = actual_output.get_output().stdout.clone();
    let actual_output = String::from_utf8(stdout)?;

    pretty_assertions::assert_eq!(actual_output, expected_output);

    Ok(())
}
