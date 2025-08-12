use assert_cmd::Command;
use assert_fs::fixture::{FileWriteStr, PathChild};
use assert_fs::prelude::PathCreateDir;
use fake::Fake;
use fake::faker::lorem::en::{Word, Words};
use predicates::prelude::predicate;
use pretty_assertions::assert_eq;

mod common;

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
    assert_eq!(bit_index_content, git_index_content);

    Ok(())
}

#[test]
fn add_multiple_files_to_index_successfully() -> Result<(), Box<dyn std::error::Error>> {
    common::redirect_temp_dir();
    let dir = assert_fs::TempDir::new()?;
    let mut cmd = Command::cargo_bin("bit")?;
    cmd.current_dir(dir.path()).arg("init");

    cmd.assert().success().stdout(predicate::str::contains(
        "Initialized empty Git repository in",
    ));

    let file_names: Vec<String> = (0..5)
        .map(|_| format!("{}.txt", Word().fake::<String>()))
        .collect();

    for file_name in &file_names {
        let file_path = dir.child(file_name.clone());
        let file_content = Words(5..10).fake::<Vec<String>>().join(" ");
        file_path.write_str(&file_content)?;
    }

    // Add the files to the index using bit
    let mut sut = Command::cargo_bin("bit")?;

    sut.current_dir(dir.path())
        .arg("add")
        .args(&file_names)
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

    // Add the files to the index using git
    let mut git_add_cmd = Command::new("git");
    git_add_cmd
        .current_dir(dir.path())
        .arg("add")
        .args(&file_names)
        .assert()
        .success();

    // Compare the index content with the git index content
    let git_index_path = dir.child(".git/index");
    let git_index_content = std::fs::read(git_index_path.path())?;
    assert_eq!(bit_index_content, git_index_content);

    Ok(())
}

#[test]
fn add_files_in_nested_directories_to_index_successfully() -> Result<(), Box<dyn std::error::Error>>
{
    common::redirect_temp_dir();
    let dir = assert_fs::TempDir::new()?;
    let mut cmd = Command::cargo_bin("bit")?;
    cmd.current_dir(dir.path()).arg("init");

    cmd.assert().success().stdout(predicate::str::contains(
        "Initialized empty Git repository in",
    ));

    // Create nested directories and files
    let mut file_names = Vec::new();
    let dir_count = (1..=5).fake::<usize>();
    for _ in 0..dir_count {
        let dir_name = Word().fake::<String>();
        let dir_path = dir.child(dir_name.clone());
        dir_path.create_dir_all()?;
        let file_count = (1..=5).fake::<usize>();
        for _ in 0..file_count {
            let file_name = format!("{}.txt", Word().fake::<String>());
            let file_path = dir_path.child(file_name.clone());
            let file_content = Words(5..10).fake::<Vec<String>>().join(" ");
            file_path.write_str(&file_content.clone())?;
            file_names.push(format!("{dir_name}/{file_name}"));
        }
    }

    // Add the files to the index using bit
    let mut sut = Command::cargo_bin("bit")?;

    sut.current_dir(dir.path())
        .arg("add")
        .arg(".")
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

    // Add the files to the index using git
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
    assert_eq!(bit_index_content, git_index_content);

    Ok(())
}

#[test]
fn add_multiple_files_to_index_incrementally_successfully() -> Result<(), Box<dyn std::error::Error>>
{
    common::redirect_temp_dir();
    let dir = assert_fs::TempDir::new()?;
    let mut cmd = Command::cargo_bin("bit")?;
    cmd.current_dir(dir.path()).arg("init");

    cmd.assert().success().stdout(predicate::str::contains(
        "Initialized empty Git repository in",
    ));

    let file_names: Vec<String> = (0..5)
        .map(|_| format!("{}.txt", Word().fake::<String>()))
        .collect();

    for file_name in &file_names {
        let file_path = dir.child(file_name.clone());
        let file_content = Words(5..10).fake::<Vec<String>>().join(" ");
        file_path.write_str(&file_content)?;
    }

    // Add the first two files to the index using bit
    let mut sut = Command::cargo_bin("bit")?;

    sut.current_dir(dir.path())
        .arg("add")
        .args(&file_names[..2])
        .assert()
        .success();

    // Add the remaining files incrementally
    let mut sut = Command::cargo_bin("bit")?;

    sut.current_dir(dir.path())
        .arg("add")
        .args(&file_names[2..])
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

    // Add the first two files to the index using git
    let mut git_add_cmd = Command::new("git");

    git_add_cmd
        .current_dir(dir.path())
        .arg("add")
        .args(&file_names[..2])
        .assert()
        .success();

    // Add the remaining files incrementally
    let mut git_add_cmd = Command::new("git");

    git_add_cmd
        .current_dir(dir.path())
        .arg("add")
        .args(&file_names[2..])
        .assert()
        .success();

    // Compare the index content with the git index content
    let git_index_path = dir.child(".git/index");
    let git_index_content = std::fs::read(git_index_path.path())?;
    assert_eq!(bit_index_content, git_index_content);

    Ok(())
}
