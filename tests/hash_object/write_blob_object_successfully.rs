use crate::common;
use assert_cmd::Command;
use assert_fs::fixture::{FileWriteStr, PathChild};
use fake::Fake;
use fake::faker::lorem::en::{Word, Words};
use predicates::prelude::predicate;

#[test]
fn write_blob_object_successfully() -> Result<(), Box<dyn std::error::Error>> {
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

    let mut sut = Command::cargo_bin("bit")?;
    sut.current_dir(dir.path())
        .arg("hash-object")
        .arg("-w")
        .arg(&file_name);

    sut.assert()
        .success()
        .stdout(predicate::str::is_match(r"^[0-9a-f]{40}$")?);

    Ok(())
}
