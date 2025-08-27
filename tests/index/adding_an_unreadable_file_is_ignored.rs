use crate::common;
use assert_cmd::Command;
use assert_fs::fixture::{FileWriteStr, PathChild};
use fake::Fake;
use fake::faker::lorem::en::{Word, Words};
use predicates::prelude::predicate;

#[test]
fn adding_an_unreadable_file_is_ignored() -> Result<(), Box<dyn std::error::Error>> {
    common::redirect_temp_dir();
    let dir = assert_fs::TempDir::new()?;
    let mut cmd = Command::cargo_bin("bit")?;
    cmd.current_dir(dir.path()).arg("init");

    cmd.assert().success().stdout(predicate::str::contains(
        "Initialized empty Git repository in",
    ));

    // Create a valid file and add it to the index
    let valid_file_name = format!("{}.txt", Word().fake::<String>());
    let valid_file_path = dir.child(valid_file_name.clone());
    let valid_file_content = Words(5..10).fake::<Vec<String>>().join(" ");
    valid_file_path.write_str(&valid_file_content)?;

    // Create an unreadable file
    let unreadable_file_name = format!("{}.txt", Word().fake::<String>());
    let unreadable_file_path = dir.child(unreadable_file_name.clone());
    let unreadable_file_content = Words(5..10).fake::<Vec<String>>().join(" ");
    unreadable_file_path.write_str(&unreadable_file_content)?;

    // Make the file unreadable (remove read permissions)
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut permissions = std::fs::metadata(unreadable_file_path.path())?.permissions();
        permissions.set_mode(0o000); // No permissions
        std::fs::set_permissions(unreadable_file_path.path(), permissions)?;
    }

    #[cfg(windows)]
    {
        // On Windows, making a file completely unreadable is more complex.
        // For simplicity, we'll skip this part of the test on Windows.
        println!("Skipping unreadable file test on Windows");
        return Ok(());
    }

    // Attempt to add both the valid file and the unreadable file to the index using bit
    let mut sut = Command::cargo_bin("bit")?;
    sut.current_dir(dir.path())
        .arg("add")
        .arg(&valid_file_name)
        .arg(&unreadable_file_name)
        .assert()
        .failure();

    // Assert that no changes were made to the index.
    // The index file should be empty
    let bit_index_path = dir.child(".git/index");
    let bit_index_content = std::fs::read(bit_index_path.path())?;
    assert!(
        bit_index_content.is_empty(),
        "Index should be empty when adding an unreadable file"
    );

    Ok(())
}
