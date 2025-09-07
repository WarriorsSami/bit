use crate::common::file::{FileSpec, write_file};
use crate::common::redirect_temp_dir;
use assert_cmd::Command;
use assert_fs::TempDir;
use derive_new::new;
use rstest::fixture;
use std::path::Path;

#[fixture]
pub fn repository_dir() -> TempDir {
    redirect_temp_dir();
    TempDir::new().expect("Failed to create temp dir")
}

#[fixture]
pub fn init_repository_dir(repository_dir: TempDir) -> TempDir {
    run_bit_command(repository_dir.path(), &["init"])
        .assert()
        .success();

    let file1 = FileSpec::new(repository_dir.path().join("1.txt"), "one".to_string());
    write_file(file1);

    let file2 = FileSpec::new(
        repository_dir.path().join("a").join("2.txt"),
        "two".to_string(),
    );
    write_file(file2);

    let file3 = FileSpec::new(
        repository_dir.path().join("a").join("b").join("3.txt"),
        "three".to_string(),
    );
    write_file(file3);

    run_bit_command(repository_dir.path(), &["add", "."])
        .assert()
        .success();

    bit_commit(repository_dir.path(), "Initial commit")
        .assert()
        .success();

    repository_dir
}

pub fn run_bit_command(dir: &Path, args: &[&str]) -> Command {
    let mut cmd = Command::cargo_bin("bit").expect("Failed to find bit binary");
    cmd.current_dir(dir);
    for arg in args {
        cmd.arg(arg);
    }
    cmd
}

pub fn run_git_command(dir: &Path, args: &[&str]) -> Command {
    let mut cmd = Command::new("git");
    cmd.current_dir(dir);
    for arg in args {
        cmd.arg(arg);
    }
    cmd
}

#[derive(Debug, Clone, new)]
struct RandomAuthor {
    name: String,
    email: String,
}

fn generate_random_author() -> RandomAuthor {
    use fake::Fake;
    use fake::faker::internet::en::FreeEmail;
    use fake::faker::name::en::Name;

    let name = Name().fake::<String>().replace(" ", "_");
    let email = FreeEmail().fake::<String>();
    RandomAuthor::new(name, email)
}

pub fn bit_commit(dir: &Path, message: &str) -> Command {
    let author = generate_random_author();
    let mut cmd = run_bit_command(dir, &["commit", "-m", message]);
    cmd.envs(vec![
        ("GIT_AUTHOR_NAME", &author.name),
        ("GIT_AUTHOR_EMAIL", &author.email),
    ]);
    cmd
}
