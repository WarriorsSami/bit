use crate::{assert_index_eq, common};
use assert_cmd::Command;
use assert_fs::fixture::{FileWriteStr, PathChild};
use fake::Fake;
use fake::faker::lorem::en::{Word, Words};
use predicates::prelude::predicate;

#[test]
fn add_single_file_to_index_successfully() -> Result<(), Box<dyn std::error::Error>> {
    common::redirect_temp_dir();
    let dir = assert_fs::TempDir::new()?;
    let mut cmd = Command::cargo_bin("bit")?;
    cmd.current_dir(dir.path()).arg("init");

    cmd.assert().success().stdout(predicate::str::contains(
        "Initialized empty Git repository in",
    ));

    let file_name = format!("{}.txt", Word().fake::<String>());
    let file_path = dir.child(file_name.clone());
    let file_content = Words(5..10).fake::<Vec<String>>().join(" ");
    file_path.write_str(&file_content.clone())?;

    // Add the file to the index using bit
    let mut sut = Command::cargo_bin("bit")?;

    sut.current_dir(dir.path())
        .arg("add")
        .arg(&file_name)
        .assert()
        .success();

    // Store the index content
    let bit_index_path = dir.child(".git/index");
    let bit_index_content = std::fs::read(bit_index_path.path())?;

    // Delete the .git directory and recreate it using git
    std::fs::remove_dir_all(dir.child(".git").path())?;
    let mut git_cmd = Command::new("git");
    git_cmd.current_dir(dir.path()).arg("init");
    git_cmd.assert().success();

    // Add the file to the index using git
    let mut git_add_cmd = Command::new("git");
    git_add_cmd
        .current_dir(dir.path())
        .arg("add")
        .arg(&file_name)
        .assert()
        .success();

    // Compare the index content with the git index content
    let git_index_path = dir.child(".git/index");
    let git_index_content = std::fs::read(git_index_path.path())?;
    assert_index_eq!(&bit_index_content, &git_index_content);

    Ok(())
}
