use crate::common::command::{bit_commit, repository_dir, run_bit_command};
use crate::common::file::{FileSpec, write_file};
use assert_fs::TempDir;
use pretty_assertions::assert_eq;
use rstest::rstest;

#[rstest]
fn list_untracked_directories_inside_tracked_directories(
    repository_dir: TempDir,
) -> Result<(), Box<dyn std::error::Error>> {
    run_bit_command(repository_dir.path(), &["init"])
        .assert()
        .success();

    let inner_file = FileSpec::new(
        repository_dir
            .path()
            .join("a")
            .join("b")
            .join("inner_file.txt"),
        String::new(),
    );
    write_file(inner_file);

    run_bit_command(repository_dir.path(), &["add", "."])
        .assert()
        .success();

    bit_commit(repository_dir.path(), "Initial commit")
        .assert()
        .success();

    let outer_file = FileSpec::new(
        repository_dir.path().join("a").join("outer_file.txt"),
        String::new(),
    );
    write_file(outer_file);

    let file = FileSpec::new(
        repository_dir
            .path()
            .join("a")
            .join("b")
            .join("c")
            .join("file.txt"),
        String::new(),
    );
    write_file(file);

    let expected_output = "?? a/b/c/\n?? a/outer_file.txt\n".to_string();
    let actual_output = run_bit_command(repository_dir.path(), &["status", "--porcelain"])
        .assert()
        .success();
    let stdout = actual_output.get_output().stdout.clone();
    let actual_output = String::from_utf8(stdout)?;

    assert_eq!(actual_output, expected_output);

    Ok(())
}
