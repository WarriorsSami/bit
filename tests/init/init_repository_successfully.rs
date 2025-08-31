use crate::common;
use assert_cmd::Command;
use predicates::prelude::predicate;

#[test]
fn init_repository_successfully() -> Result<(), Box<dyn std::error::Error>> {
    common::redirect_temp_dir();
    let dir = assert_fs::TempDir::new()?;
    let dir_absolute_path = dir.path().canonicalize()?.display().to_string();
    let mut sut = Command::cargo_bin("bit")?;

    sut.arg("init").arg(dir.path());

    sut.assert()
        .success()
        .stdout(predicate::str::is_match(
            r"^Initialized empty Git repository in .+$",
        )?)
        .stdout(predicate::str::contains(dir_absolute_path));

    Ok(())
}
