use crate::{assert_index_eq, common};
use assert_cmd::Command;
use assert_fs::fixture::{FileWriteStr, PathChild, PathCreateDir};
use fake::Fake;
use fake::faker::lorem::en::{Word, Words};
use predicates::prelude::predicate;

#[test]
fn replace_directory_having_nested_children_with_file_successfully()
-> Result<(), Box<dyn std::error::Error>> {
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

    // Create a directory and add a nested directory with a file inside it
    let dir_name = format!("{}.dir", Word().fake::<String>());
    let dir_path = dir.child(dir_name.clone());
    dir_path.create_dir_all()?;

    let nested_dir_name = format!("nested_{}.dir", Word().fake::<String>());
    let nested_dir_path = dir_path.child(nested_dir_name.clone());
    nested_dir_path.create_dir_all()?;

    let nested_file_name = format!("nested_file_{}.txt", Word().fake::<String>());
    let nested_file_path = nested_dir_path.child(nested_file_name.clone());
    let nested_file_content = Words(5..10).fake::<Vec<String>>().join(" ");
    nested_file_path.write_str(&nested_file_content)?;

    // Add the nested file to the index using bit
    let mut sut = Command::cargo_bin("bit")?;
    sut.current_dir(dir.path())
        .arg("add")
        .arg(nested_file_path.path())
        .assert()
        .success();

    // Remove the directory and create a file with the same name
    std::fs::remove_dir_all(dir_path.path())?;
    let new_file_path = dir.child(dir_name.clone());
    new_file_path.write_str(&nested_file_content)?;

    // Attempt to add the new file to the index using bit
    let mut sut = Command::cargo_bin("bit")?;
    sut.current_dir(dir.path())
        .arg("add")
        .arg(new_file_path.path())
        .assert()
        .success();

    // Store the index content
    let bit_index_path = dir.child(".git/index");
    let bit_index_content = std::fs::read(bit_index_path.path())?;

    // Delete the .git directory and recreate it using git
    std::fs::remove_dir_all(dir.child(".git").path())?;
    let mut git_cmd = Command::new("git");
    git_cmd
        .current_dir(dir.path())
        .arg("init")
        .assert()
        .success();

    // Add the new file to the index using git
    let mut git_add_cmd = Command::new("git");
    git_add_cmd
        .current_dir(dir.path())
        .arg("add")
        .arg(".")
        .assert()
        .success();

    // Compare the index content with the git index content
    let git_index_path = dir.child(".git/index");
    let git_index_content = std::fs::read(git_index_path.path())?;
    assert_index_eq!(&bit_index_content, &git_index_content);

    Ok(())
}
