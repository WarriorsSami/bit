use crate::common::command::init_repository_dir;
use assert_fs::TempDir;
use rstest::rstest;

#[rstest]
fn list_all_blobs_from_head_commit(
    init_repository_dir: TempDir,
) -> Result<(), Box<dyn std::error::Error>> {
    let repository_dir = init_repository_dir;

    let expected_output = "100644 43dd47ea691c90a5fa7827892c70241913351963 1.txt\n100644 64c5e5885a4b06010b3a0c20edb7900dd0311025 a/2.txt\n100644 1d19714ffbc272ba0da6eb419d66123c20527174 a/b/3.txt\n".to_string();
    let actual_output =
        crate::common::command::run_bit_command(repository_dir.path(), &["ls-tree", "-r", "HEAD"])
            .assert()
            .success();
    let stdout = actual_output.get_output().stdout.clone();
    let actual_output = String::from_utf8(stdout)?;

    pretty_assertions::assert_eq!(actual_output, expected_output);

    Ok(())
}
