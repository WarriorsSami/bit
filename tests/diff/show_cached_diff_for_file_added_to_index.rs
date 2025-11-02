use crate::common::command::{init_repository_dir, run_bit_command};
use crate::common::file::{FileSpec, write_file};
use assert_fs::TempDir;
use rstest::rstest;

#[rstest]
fn show_cached_diff_for_file_added_to_index(
    init_repository_dir: TempDir,
) -> Result<(), Box<dyn std::error::Error>> {
    let repository_dir = init_repository_dir;

    // create a new file 4.txt that doesn't exist in HEAD
    let file4_spec = FileSpec::new(repository_dir.path().join("4.txt"), "four".to_string());
    write_file(file4_spec);

    // add the new file to the index
    run_bit_command(repository_dir.path(), &["add", "4.txt"])
        .assert()
        .success();

    let expected_output =
        "diff --git a/4.txt b/4.txt\nnew file mode 100644\nindex 0000000..ea1f343\n--- /dev/null\n+++ b/4.txt\n@@ -1,0 +1,1 @@\n+four\n"
            .to_string();
    let actual_output = run_bit_command(repository_dir.path(), &["diff", "--cached"])
        .assert()
        .success();
    let stdout = actual_output.get_output().stdout.clone();
    let actual_output = String::from_utf8(stdout)?;

    pretty_assertions::assert_eq!(actual_output, expected_output);

    Ok(())
}
