use crate::common::command::{bit_commit, get_head_commit_sha, run_bit_command};
use crate::common::file::{FileSpec, write_file};
use assert_fs::TempDir;
use rstest::rstest;

#[rstest]
fn show_log_with_patch(
    #[from(crate::common::command::repository_dir)] repository_dir: TempDir,
) -> Result<(), Box<dyn std::error::Error>> {
    // Initialize repository
    run_bit_command(repository_dir.path(), &["init"])
        .assert()
        .success();

    // First commit - create initial file
    let file1 = FileSpec::new(
        repository_dir.path().join("file1.txt"),
        "line 1\nline 2\nline 3\n".to_string(),
    );
    write_file(file1);
    run_bit_command(repository_dir.path(), &["add", "."])
        .assert()
        .success();
    bit_commit(repository_dir.path(), "First commit - add file1")
        .assert()
        .success();

    let commit1_sha = get_head_commit_sha(repository_dir.path())?;

    // Second commit - modify existing file and add new file
    let file1_modified = FileSpec::new(
        repository_dir.path().join("file1.txt"),
        "line 1 modified\nline 2\nline 3\nline 4\n".to_string(),
    );
    write_file(file1_modified);

    let file2 = FileSpec::new(
        repository_dir.path().join("file2.txt"),
        "new file content\n".to_string(),
    );
    write_file(file2);

    run_bit_command(repository_dir.path(), &["add", "."])
        .assert()
        .success();
    bit_commit(repository_dir.path(), "Second commit - modify and add")
        .assert()
        .success();

    let commit2_sha = get_head_commit_sha(repository_dir.path())?;

    // Third commit - modify file2
    let file2_modified = FileSpec::new(
        repository_dir.path().join("file2.txt"),
        "new file content\nmore content\n".to_string(),
    );
    write_file(file2_modified);

    run_bit_command(repository_dir.path(), &["add", "."])
        .assert()
        .success();
    bit_commit(repository_dir.path(), "Third commit - update file2")
        .assert()
        .success();

    let commit3_sha = get_head_commit_sha(repository_dir.path())?;

    // Run log with --patch flag
    let output = run_bit_command(
        repository_dir.path(),
        &["log", "--patch", "--decorate=none"],
    )
    .assert()
    .success();

    let stdout = String::from_utf8(output.get_output().stdout.clone())?;

    // Verify commit headers are present (in reverse chronological order)
    assert!(stdout.contains(&format!("commit {}", commit3_sha)));
    assert!(stdout.contains(&format!("commit {}", commit2_sha)));
    assert!(stdout.contains(&format!("commit {}", commit1_sha)));

    assert!(stdout.contains("Author: fake_user <fake_email@email.com>"));
    assert!(stdout.contains("Third commit - update file2"));
    assert!(stdout.contains("Second commit - modify and add"));
    assert!(stdout.contains("First commit - add file1"));

    // Verify patches for commit 3 (most recent)
    assert!(stdout.contains("diff --git a/file2.txt b/file2.txt"));
    assert!(stdout.contains("+more content"));

    // Verify patches for commit 2
    assert!(stdout.contains("diff --git a/file1.txt b/file1.txt"));
    assert!(stdout.contains("-line 1"));
    assert!(stdout.contains("+line 1 modified"));
    assert!(stdout.contains("+line 4"));

    assert!(stdout.contains("diff --git a/file2.txt b/file2.txt"));
    assert!(stdout.contains("new file mode"));
    assert!(stdout.contains("+new file content"));

    // Verify patches for commit 1 (initial commit)
    // Initial commit shows all content as additions
    assert!(stdout.contains("+line 1"));
    assert!(stdout.contains("+line 2"));
    assert!(stdout.contains("+line 3"));

    // Verify the order: commit 3 patch, then commit 2 patch, then commit 1 patch
    let commit3_pos = stdout.find(&format!("commit {}", commit3_sha)).unwrap();
    let commit2_pos = stdout.find(&format!("commit {}", commit2_sha)).unwrap();
    let commit1_pos = stdout.find(&format!("commit {}", commit1_sha)).unwrap();

    assert!(
        commit3_pos < commit2_pos,
        "Commit 3 should appear before commit 2"
    );
    assert!(
        commit2_pos < commit1_pos,
        "Commit 2 should appear before commit 1"
    );

    Ok(())
}
