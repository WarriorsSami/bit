use crate::common;
use assert_cmd::Command;
use assert_fs::fixture::{FileWriteStr, PathChild};
use fake::Fake;
use fake::faker::lorem::en::{Word, Words};
use predicates::prelude::predicate;

#[test]
fn adding_a_non_existent_file_is_ignored() -> Result<(), Box<dyn std::error::Error>> {
    common::redirect_temp_dir();
    let dir = assert_fs::TempDir::new()?;
    let mut cmd = Command::cargo_bin("bit")?;
    cmd.current_dir(dir.path()).arg("init");

    cmd.assert().success().stdout(predicate::str::contains(
        "Initialized empty Git repository in",
    ));

    // Create a valid file and add it to the add
    let valid_file_name = format!("{}.txt", Word().fake::<String>());
    let valid_file_path = dir.child(valid_file_name.clone());
    let valid_file_content = Words(5..10).fake::<Vec<String>>().join(" ");
    valid_file_path.write_str(&valid_file_content)?;

    // Define a non-existent file name
    let non_existent_file_name = format!("{}.txt", Word().fake::<String>());

    // Attempt to add both the valid file and the non-existent file to the add using bit
    let mut sut = Command::cargo_bin("bit")?;
    sut.current_dir(dir.path())
        .arg("add")
        .arg(&valid_file_name)
        .arg(&non_existent_file_name)
        .assert()
        .failure();

    Ok(())
}
