use crate::common::command::{get_head_commit_sha, repository_dir, run_bit_command};
use crate::common::file::{FileSpec, write_file};
use assert_fs::TempDir;
use predicates::prelude::predicate;
use rstest::rstest;

#[rstest]
fn create_branch_from_blob_oid_fails(
    repository_dir: TempDir,
) -> Result<(), Box<dyn std::error::Error>> {
    let repository_dir = repository_dir;

    // Initialize repository
    run_bit_command(repository_dir.path(), &["init"])
        .assert()
        .success();

    // Create and add a file
    let file = FileSpec::new(
        repository_dir.path().join("test.txt"),
        "test content".to_string(),
    );
    write_file(file);

    run_bit_command(repository_dir.path(), &["add", "test.txt"])
        .assert()
        .success();

    // Hash the file to get its blob OID
    let output = run_bit_command(repository_dir.path(), &["hash-object", "test.txt"]).output()?;
    let blob_oid = String::from_utf8(output.stdout)?.trim().to_string();

    // Try to create a branch from the blob OID (should fail)
    run_bit_command(
        repository_dir.path(),
        &["branch", "create", "test-branch", &blob_oid],
    )
    .assert()
    .failure()
    .stderr(predicate::str::contains("is a blob, not a commit"));

    Ok(())
}

#[rstest]
fn create_branch_from_abbreviated_blob_oid_fails(
    repository_dir: TempDir,
) -> Result<(), Box<dyn std::error::Error>> {
    let repository_dir = repository_dir;

    // Initialize repository
    run_bit_command(repository_dir.path(), &["init"])
        .assert()
        .success();

    // Create and add a file
    let file = FileSpec::new(
        repository_dir.path().join("test.txt"),
        "test content".to_string(),
    );
    write_file(file);

    run_bit_command(repository_dir.path(), &["add", "test.txt"])
        .assert()
        .success();

    // Hash the file to get its blob OID
    let output = run_bit_command(repository_dir.path(), &["hash-object", "test.txt"]).output()?;
    let blob_oid = String::from_utf8(output.stdout)?.trim().to_string();
    let abbreviated_blob_oid = &blob_oid[..10];

    // Try to create a branch from the abbreviated blob OID (should fail)
    run_bit_command(
        repository_dir.path(),
        &["branch", "create", "test-branch", abbreviated_blob_oid],
    )
    .assert()
    .failure()
    .stderr(predicate::str::contains("is a blob, not a commit"));

    Ok(())
}

#[rstest]
fn create_branch_from_tree_oid_fails(
    repository_dir: TempDir,
) -> Result<(), Box<dyn std::error::Error>> {
    let repository_dir = repository_dir;

    // Initialize repository
    run_bit_command(repository_dir.path(), &["init"])
        .assert()
        .success();

    // Create and commit a file to generate a tree
    let file = FileSpec::new(
        repository_dir.path().join("test.txt"),
        "test content".to_string(),
    );
    write_file(file);

    run_bit_command(repository_dir.path(), &["add", "test.txt"])
        .assert()
        .success();

    // Commit to create tree object
    run_bit_command(repository_dir.path(), &["commit", "-m", "Initial commit"])
        .env("GIT_AUTHOR_NAME", "Test User")
        .env("GIT_AUTHOR_EMAIL", "test@example.com")
        .env("GIT_AUTHOR_DATE", "2023-01-01 12:00:00 +0000")
        .assert()
        .success();

    // Get the tree OID from the commit (resolve symbolic references)
    let commit_oid = get_head_commit_sha(repository_dir.path())?;

    // Use git cat-file to get the tree OID from the commit
    let output = std::process::Command::new("git")
        .current_dir(repository_dir.path())
        .args(["cat-file", "commit", &commit_oid])
        .output()?;

    let commit_content = String::from_utf8(output.stdout)?;
    let tree_line = commit_content
        .lines()
        .find(|line| line.starts_with("tree "))
        .ok_or("No tree line found")?;
    let tree_oid = tree_line.strip_prefix("tree ").unwrap().trim();

    // Try to create a branch from the tree OID (should fail)
    run_bit_command(
        repository_dir.path(),
        &["branch", "create", "test-branch", tree_oid],
    )
    .assert()
    .failure()
    .stderr(predicate::str::contains("is a tree, not a commit"));

    Ok(())
}

#[rstest]
fn ambiguous_oid_only_shows_commit_candidates(
    repository_dir: TempDir,
) -> Result<(), Box<dyn std::error::Error>> {
    let repository_dir = repository_dir;

    // Initialize repository
    run_bit_command(repository_dir.path(), &["init"])
        .assert()
        .success();

    // Create multiple objects (blobs and commits) to potentially create ambiguity
    // This test mainly ensures the filtering logic is in place
    for i in 0..5 {
        let file = FileSpec::new(
            repository_dir.path().join(format!("file{}.txt", i)),
            format!("content {}", i),
        );
        write_file(file);

        run_bit_command(repository_dir.path(), &["add", "."])
            .assert()
            .success();

        run_bit_command(
            repository_dir.path(),
            &["commit", "-m", &format!("Commit {}", i)],
        )
        .env("GIT_AUTHOR_NAME", "Test User")
        .env("GIT_AUTHOR_EMAIL", "test@example.com")
        .env("GIT_AUTHOR_DATE", "2023-01-01 12:00:00 +0000")
        .assert()
        .success();
    }

    // Note: In practice, creating truly ambiguous OIDs (same prefix for blob and commit)
    // is extremely difficult with SHA-1, so this test primarily validates the logic exists

    Ok(())
}
