use assert_cmd::prelude::{CommandCargoExt, OutputAssertExt};
use assert_fs::prelude::{FileWriteStr, PathChild};
use fake::faker::internet::en::FreeEmail;
use fake::faker::lorem::en::{Word, Words};
use fake::faker::name::en::Name;
use fake::Fake;
use predicates::prelude::predicate;
use std::process::Command;

const TMPDIR: &str = "../playground";

fn redirect_temp_dir() {
    std::env::set_var("TMPDIR", TMPDIR);
}

#[test]
fn new_repository_initiated_with_git_directory() -> Result<(), Box<dyn std::error::Error>> {
    redirect_temp_dir();
    let dir = assert_fs::TempDir::new()?;
    let dir_absolute_path = dir.path().canonicalize()?.display().to_string();
    let mut sut = Command::cargo_bin("bit")?;

    sut.arg("init").arg(dir.path());

    sut.assert()
        .success()
        .stdout(predicate::str::is_match(
            r"^Initialized git directory at .+$",
        )?)
        .stdout(predicate::str::contains(dir_absolute_path));

    Ok(())
}

#[test]
fn read_blob_object_successfully() -> Result<(), Box<dyn std::error::Error>> {
    redirect_temp_dir();
    let dir = assert_fs::TempDir::new()?;
    let mut cmd = Command::cargo_bin("bit")?;
    cmd.current_dir(dir.path()).arg("init");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Initialized git directory"));

    let file_name = format!("{}.txt", Word().fake::<String>());
    let file_path = dir.child(file_name.clone());
    let file_content = Words(5..10).fake::<Vec<String>>().join(" ");
    file_path.write_str(&file_content.clone())?;

    let mut git_cmd = Command::new("git");
    let blob_sha_raw = git_cmd
        .current_dir(dir.path())
        .arg("hash-object")
        .arg("-w")
        .arg(&file_name)
        .output()?
        .stdout
        .trim_ascii()
        .to_vec();
    let blob_sha = String::from_utf8(blob_sha_raw)?;

    let mut git_cmd = Command::new("git");
    let file_content_raw = git_cmd
        .current_dir(dir.path())
        .arg("cat-file")
        .arg("-p")
        .arg(&blob_sha)
        .output()?
        .stdout
        .trim_ascii()
        .to_vec();
    let file_content = String::from_utf8(file_content_raw)?;

    let mut sut = Command::cargo_bin("bit")?;
    sut.current_dir(dir.path())
        .arg("cat-file")
        .arg("-p")
        .arg(&blob_sha);

    sut.assert().success().stdout(predicate::eq(file_content));

    Ok(())
}

#[test]
fn write_blob_object_successfully() -> Result<(), Box<dyn std::error::Error>> {
    redirect_temp_dir();
    let dir = assert_fs::TempDir::new()?;
    let mut cmd = Command::cargo_bin("bit")?;
    cmd.current_dir(dir.path()).arg("init");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Initialized git directory"));

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

#[test]
fn write_commit_object_successfully() -> Result<(), Box<dyn std::error::Error>> {
    redirect_temp_dir();
    let dir = assert_fs::TempDir::new()?;
    let mut cmd = Command::cargo_bin("bit")?;
    cmd.current_dir(dir.path()).arg("init");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Initialized git directory"));

    // create a few files (random number between 1 and 5) and write random content to them
    let file_count = (1..=5).fake::<usize>();
    let mut file_names = Vec::new();
    for _ in 0..file_count {
        let file_name = format!("{}.txt", Word().fake::<String>());
        let file_path = dir.child(file_name.clone());
        let file_content = Words(5..10).fake::<Vec<String>>().join(" ");
        file_path.write_str(&file_content.clone())?;
        file_names.push(file_name);
    }
    
    // create fake author config and message
    let author_name = Name().fake::<String>();
    let author_email = FreeEmail().fake::<String>();
    let message = Words(5..10).fake::<Vec<String>>().join("\n"); 

    // commit the files using bit
    let mut sut = Command::cargo_bin("bit")?;
    sut.current_dir(dir.path())
        .envs(vec![("GIT_AUTHOR_NAME", author_name), ("GIT_AUTHOR_EMAIL", author_email)])
        .arg("commit")
        .arg("-m")
        .arg(message);

    // assert that the commit was successful
    sut.assert()
        .success()
        .stdout(predicate::str::is_match(r"^root-commit: [0-9a-f]{40}\n.+$")?);

    Ok(())
}
