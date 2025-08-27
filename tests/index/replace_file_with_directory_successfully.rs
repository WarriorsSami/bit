use crate::{assert_index_eq, common};
use assert_cmd::Command;
use assert_fs::fixture::{FileWriteStr, PathChild, PathCreateDir};
use fake::Fake;
use fake::faker::lorem::en::{Word, Words};
use predicates::prelude::predicate;

#[test]
fn replace_file_with_directory_successfully() -> Result<(), Box<dyn std::error::Error>> {
    common::redirect_temp_dir();
    let dir = assert_fs::TempDir::new()?;
    let mut cmd = Command::cargo_bin("bit")?;
    cmd.current_dir(dir.path()).arg("init");

    cmd.assert().success().stdout(predicate::str::contains(
        "Initialized empty Git repository in",
    ));

    // Create a file and add it to the index
    let file_name = format!("{}.txt", Word().fake::<String>());
    let file_path = dir.child(file_name.clone());
    let file_content = Words(5..10).fake::<Vec<String>>().join(" ");
    file_path.write_str(&file_content)?;

    // Add the file to the index using bit
    let mut sut = Command::cargo_bin("bit")?;
    sut.current_dir(dir.path())
        .arg("add")
        .arg(&file_name)
        .assert()
        .success();

    // Create a directory with the same name as the file
    let dir_name = file_name;
    let dir_path = dir.child(dir_name.clone());

    // Remove the file and create a directory with the same name
    std::fs::remove_file(file_path.path())?;

    // Create the directory and add a file inside it
    dir_path.create_dir_all()?;
    let nested_file_name = format!("nested_{}.txt", Word().fake::<String>());
    let nested_file_path = dir_path.child(nested_file_name.clone());
    let nested_file_content = Words(5..10).fake::<Vec<String>>().join(" ");
    nested_file_path.write_str(&nested_file_content)?;

    // Attempt to add the directory to the index using bit
    let mut sut = Command::cargo_bin("bit")?;
    sut.current_dir(dir.path())
        .arg("add")
        .arg(&dir_name)
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

    // Add the directory to the index using git
    let mut git_add_cmd = Command::new("git");
    git_add_cmd
        .current_dir(dir.path())
        .arg("add")
        .arg(&dir_name)
        .assert()
        .success();

    // Compare the index content with the git index content
    let git_index_path = dir.child(".git/index");
    let git_index_content = std::fs::read(git_index_path.path())?;
    assert_index_eq!(&bit_index_content, &git_index_content);

    Ok(())
}
