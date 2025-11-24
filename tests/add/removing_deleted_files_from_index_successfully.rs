use crate::common::command::{bit_commit, run_bit_command};
use crate::common::file::{FileSpec, delete_path, write_file};
use assert_fs::TempDir;
use rstest::rstest;

#[rstest]
fn removing_deleted_files_from_index_successfully(
    #[from(crate::common::command::repository_dir)] repository_dir: TempDir,
) -> Result<(), Box<dyn std::error::Error>> {
    // Initialize repository
    run_bit_command(repository_dir.path(), &["init"])
        .assert()
        .success();

    // Create initial files
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

    // Add all files to index
    run_bit_command(repository_dir.path(), &["add", "."])
        .assert()
        .success();

    // Verify all files are in the index
    let status_output = run_bit_command(repository_dir.path(), &["status", "--porcelain"])
        .assert()
        .success();

    let stdout = String::from_utf8(status_output.get_output().stdout.clone())?;
    assert!(stdout.contains("A  1.txt"));
    assert!(stdout.contains("A  a/2.txt"));
    assert!(stdout.contains("A  a/b/3.txt"));

    // Commit the files
    bit_commit(repository_dir.path(), "Initial commit")
        .assert()
        .success();

    // Now delete a/2.txt from the workspace
    delete_path(repository_dir.path().join("a").join("2.txt").as_path());

    // Add new file to ensure we're not just removing everything
    let file4 = FileSpec::new(repository_dir.path().join("4.txt"), "four".to_string());
    write_file(file4);

    // Stage changes with git add .
    // This should:
    // 1. Remove a/2.txt from the index (since it's deleted from workspace)
    // 2. Add 4.txt to the index (since it's new)
    run_bit_command(repository_dir.path(), &["add", "."])
        .assert()
        .success();

    // Verify the status shows the correct changes
    let status_output = run_bit_command(repository_dir.path(), &["status", "--porcelain"])
        .assert()
        .success();

    let stdout = String::from_utf8(status_output.get_output().stdout.clone())?;

    // a/2.txt should be marked as deleted
    assert!(
        stdout.contains("D  a/2.txt"),
        "Expected 'D  a/2.txt' in status output, got:\n{}",
        stdout
    );

    // 4.txt should be marked as added
    assert!(
        stdout.contains("A  4.txt"),
        "Expected 'A  4.txt' in status output, got:\n{}",
        stdout
    );

    // 1.txt and a/b/3.txt should not appear (unchanged)
    assert!(
        !stdout.contains("1.txt") || stdout.contains("?? 1.txt"),
        "1.txt should not appear in staged changes"
    );
    assert!(
        !stdout.contains("a/b/3.txt") || stdout.contains("?? a/b/3.txt"),
        "a/b/3.txt should not appear in staged changes"
    );

    // Commit the changes
    bit_commit(repository_dir.path(), "Delete a/2.txt and add 4.txt")
        .assert()
        .success();

    // Verify the final state by listing files in HEAD
    let ls_files_output = run_bit_command(repository_dir.path(), &["ls-tree", "-r", "HEAD"])
        .assert()
        .success();

    let stdout = String::from_utf8(ls_files_output.get_output().stdout.clone())?;

    // a/2.txt should NOT be in the commit
    assert!(
        !stdout.contains("a/2.txt"),
        "a/2.txt should not be in HEAD after deletion, got:\n{}",
        stdout
    );

    // 4.txt should be in the commit
    assert!(
        stdout.contains("4.txt"),
        "4.txt should be in HEAD, got:\n{}",
        stdout
    );

    // Other files should still be there
    assert!(stdout.contains("1.txt"), "1.txt should be in HEAD");
    assert!(stdout.contains("a/b/3.txt"), "a/b/3.txt should be in HEAD");

    Ok(())
}

#[rstest]
fn removing_multiple_deleted_files_from_nested_directories(
    #[from(crate::common::command::repository_dir)] repository_dir: TempDir,
) -> Result<(), Box<dyn std::error::Error>> {
    // Initialize repository
    run_bit_command(repository_dir.path(), &["init"])
        .assert()
        .success();

    // Create nested structure with multiple files
    let files = vec![
        ("1.txt", "one"),
        ("a/2.txt", "two"),
        ("a/3.txt", "three"),
        ("a/b/4.txt", "four"),
        ("a/b/5.txt", "five"),
        ("c/6.txt", "six"),
    ];

    for (path, content) in &files {
        let file = FileSpec::new(repository_dir.path().join(path), content.to_string());
        write_file(file);
    }

    // Add and commit all files
    run_bit_command(repository_dir.path(), &["add", "."])
        .assert()
        .success();

    bit_commit(repository_dir.path(), "Initial commit with nested files")
        .assert()
        .success();

    // Delete multiple files from different directories
    delete_path(repository_dir.path().join("a").join("2.txt").as_path());
    delete_path(
        repository_dir
            .path()
            .join("a")
            .join("b")
            .join("4.txt")
            .as_path(),
    );
    delete_path(repository_dir.path().join("c").join("6.txt").as_path());

    // Stage the deletions
    run_bit_command(repository_dir.path(), &["add", "."])
        .assert()
        .success();

    // Verify status shows all deletions
    let status_output = run_bit_command(repository_dir.path(), &["status", "--porcelain"])
        .assert()
        .success();

    let stdout = String::from_utf8(status_output.get_output().stdout.clone())?;

    assert!(
        stdout.contains("D  a/2.txt"),
        "Expected deletion of a/2.txt, got:\n{}",
        stdout
    );
    assert!(
        stdout.contains("D  a/b/4.txt"),
        "Expected deletion of a/b/4.txt, got:\n{}",
        stdout
    );
    assert!(
        stdout.contains("D  c/6.txt"),
        "Expected deletion of c/6.txt, got:\n{}",
        stdout
    );

    // Commit the deletions
    bit_commit(repository_dir.path(), "Delete multiple files")
        .assert()
        .success();

    // Verify deleted files are not in HEAD
    let ls_files_output = run_bit_command(repository_dir.path(), &["ls-tree", "-r", "HEAD"])
        .assert()
        .success();

    let stdout = String::from_utf8(ls_files_output.get_output().stdout.clone())?;

    assert!(!stdout.contains("a/2.txt"), "a/2.txt should be deleted");
    assert!(!stdout.contains("a/b/4.txt"), "a/b/4.txt should be deleted");
    assert!(!stdout.contains("c/6.txt"), "c/6.txt should be deleted");

    // Verify remaining files are still there
    assert!(stdout.contains("1.txt"), "1.txt should remain");
    assert!(stdout.contains("a/3.txt"), "a/3.txt should remain");
    assert!(stdout.contains("a/b/5.txt"), "a/b/5.txt should remain");

    Ok(())
}

#[rstest]
fn removing_deleted_file_with_specific_path_argument(
    #[from(crate::common::command::repository_dir)] repository_dir: TempDir,
) -> Result<(), Box<dyn std::error::Error>> {
    // Initialize repository
    run_bit_command(repository_dir.path(), &["init"])
        .assert()
        .success();

    // Create files in different directories
    let file1 = FileSpec::new(repository_dir.path().join("1.txt"), "one".to_string());
    write_file(file1);

    let file2 = FileSpec::new(
        repository_dir.path().join("a").join("2.txt"),
        "two".to_string(),
    );
    write_file(file2);

    let file3 = FileSpec::new(
        repository_dir.path().join("b").join("3.txt"),
        "three".to_string(),
    );
    write_file(file3);

    // Add and commit
    run_bit_command(repository_dir.path(), &["add", "."])
        .assert()
        .success();

    bit_commit(repository_dir.path(), "Initial commit")
        .assert()
        .success();

    // Delete files from both directories
    delete_path(repository_dir.path().join("a").join("2.txt").as_path());
    delete_path(repository_dir.path().join("b").join("3.txt").as_path());

    // Stage only the 'a' directory
    run_bit_command(repository_dir.path(), &["add", "a"])
        .assert()
        .success();

    // Verify status shows only a/2.txt as deleted
    let status_output = run_bit_command(repository_dir.path(), &["status", "--porcelain"])
        .assert()
        .success();

    let stdout = String::from_utf8(status_output.get_output().stdout.clone())?;

    // a/2.txt should be staged for deletion
    assert!(
        stdout.contains("D  a/2.txt"),
        "Expected 'D  a/2.txt' to be staged, got:\n{}",
        stdout
    );

    // b/3.txt should NOT be staged (only workspace change)
    assert!(
        stdout.contains(" D b/3.txt") || stdout.contains("?D b/3.txt"),
        "b/3.txt should show as unstaged deletion, got:\n{}",
        stdout
    );

    Ok(())
}
