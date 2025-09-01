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
