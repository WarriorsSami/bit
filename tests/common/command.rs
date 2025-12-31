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

#[fixture]
pub fn file_a() -> String {
    r#"fn main() {
    let s = String::new();
    std::io::stdin().read_line(&mut s).unwrap();
    for i in 0..1000000000 {
        println!("{}",  s);
    }

    println!("Done");

    let tx = std::thread::spawn(move || {
        for i in 0..10 {
            println!("Thread: {}", i);
        }
    });

    tx.join().unwrap();

    println!("All threads completed");
}"#
    .to_string()
}

#[fixture]
pub fn file_b() -> String {
    r#"fn main() {
    let s = String::new();
    std::io::stdin().read_line(&mut s).unwrap();

    println!("Done");

    let tx = std::thread::spawn(move || {
        for i in 0..10 {
            println!("Thread: {}", i);
        }
    });

    if let Err(e) = tx.join() {
        eprintln!("Thread error: {}", e);
    }

    println!("All threads completed");
}"#
    .to_string()
}

#[fixture]
pub fn diff_hunks_output() -> String {
    "@@ -1,9 +1,6 @@\n fn main() {\n     let s = String::new();\n     std::io::stdin().read_line(&mut s).unwrap();\n-    for i in 0..1000000000 {\n-        println!(\"{}\",  s);\n-    }\n \n     println!(\"Done\");\n \n@@ -13,7 +10,9 @@\n         }\n     });\n \n-    tx.join().unwrap();\n+    if let Err(e) = tx.join() {\n+        eprintln!(\"Thread error: {}\", e);\n+    }\n \n     println!(\"All threads completed\");\n }\n"
    .to_string()
}

#[fixture]
pub fn init_repository_dir_for_diff_hunks(repository_dir: TempDir, file_a: String) -> TempDir {
    run_bit_command(repository_dir.path(), &["init"])
        .assert()
        .success();

    let file1 = FileSpec::new(repository_dir.path().join("1.txt"), file_a.clone());
    write_file(file1);

    let file2 = FileSpec::new(
        repository_dir.path().join("a").join("2.txt"),
        file_a.clone(),
    );
    write_file(file2);

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
    cmd.envs(vec![("NO_PAGER", "1")]);
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
    let mut cmd = run_bit_command(dir, &["commit", "-m", message]);
    cmd.envs(vec![
        ("GIT_AUTHOR_NAME", &"fake_user".to_string()),
        ("GIT_AUTHOR_EMAIL", &"fake_email@email.com".to_string()),
        ("GIT_AUTHOR_DATE", &"2023-01-01 12:00:00 +0000".to_string()), // %Y-%m-%d %H:%M:%S %z
    ]);
    cmd
}

/// Get the parent commit ID of a given commit by using git cat-file
pub fn get_parent_commit_id(
    dir: &Path,
    commit_id: &str,
) -> Result<String, Box<dyn std::error::Error>> {
    let output = run_git_command(dir, &["cat-file", "commit", commit_id]).output()?;

    let stdout = String::from_utf8(output.stdout)?;

    // Find the parent line
    for line in stdout.lines() {
        if let Some(oid) = line.strip_prefix("parent ") {
            return Ok(oid.to_string());
        }
    }

    Err("No parent found".into())
}

/// Get the Nth ancestor of a commit
pub fn get_ancestor_commit_id(
    dir: &Path,
    commit_id: &str,
    generations: usize,
) -> Result<String, Box<dyn std::error::Error>> {
    let mut current = commit_id.to_string();
    for _ in 0..generations {
        current = get_parent_commit_id(dir, &current)?;
    }
    Ok(current)
}

/// Get the current HEAD commit SHA
pub fn get_head_commit_sha(dir: &Path) -> Result<String, Box<dyn std::error::Error>> {
    let head_path = dir.join(".git").join("HEAD");
    let head_content = std::fs::read_to_string(head_path)?;

    // HEAD file contains either a commit SHA or a ref like "ref: refs/heads/main"
    if let Some(ref_path) = head_content.strip_prefix("ref: ") {
        let ref_file = dir.join(".git").join(ref_path.trim());
        let commit_sha = std::fs::read_to_string(ref_file)?;
        Ok(commit_sha.trim().to_string())
    } else {
        Ok(head_content.trim().to_string())
    }
}

#[fixture]
pub fn repository_with_multiple_commits(repository_dir: TempDir) -> TempDir {
    run_bit_command(repository_dir.path(), &["init"])
        .assert()
        .success();

    // First commit
    let file1 = FileSpec::new(
        repository_dir.path().join("file1.txt"),
        "content 1".to_string(),
    );
    write_file(file1);
    run_bit_command(repository_dir.path(), &["add", "."])
        .assert()
        .success();
    bit_commit(repository_dir.path(), "First commit")
        .assert()
        .success();

    // Second commit
    let file2 = FileSpec::new(
        repository_dir.path().join("file2.txt"),
        "content 2".to_string(),
    );
    write_file(file2);
    run_bit_command(repository_dir.path(), &["add", "."])
        .assert()
        .success();
    bit_commit(repository_dir.path(), "Second commit")
        .assert()
        .success();

    // Third commit
    let file3 = FileSpec::new(
        repository_dir.path().join("file3.txt"),
        "content 3".to_string(),
    );
    write_file(file3);
    run_bit_command(repository_dir.path(), &["add", "."])
        .assert()
        .success();
    bit_commit(repository_dir.path(), "Third commit")
        .assert()
        .success();

    // Fourth commit
    let file4 = FileSpec::new(
        repository_dir.path().join("file4.txt"),
        "content 4".to_string(),
    );
    write_file(file4);
    run_bit_command(repository_dir.path(), &["add", "."])
        .assert()
        .success();
    bit_commit(repository_dir.path(), "Fourth commit")
        .assert()
        .success();

    repository_dir
}
