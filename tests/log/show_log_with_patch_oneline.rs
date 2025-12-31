use crate::common::command::{bit_commit, get_head_commit_sha, run_bit_command};
use crate::common::file::{FileSpec, write_file};
use assert_fs::TempDir;
use rstest::rstest;

#[rstest]
fn show_log_with_patch_oneline(
    #[from(crate::common::command::repository_dir)] repository_dir: TempDir,
) -> Result<(), Box<dyn std::error::Error>> {
    // Initialize repository
    run_bit_command(repository_dir.path(), &["init"])
        .assert()
        .success();

    // First commit - create initial file
    let file1 = FileSpec::new(
        repository_dir.path().join("test.txt"),
        "original content\n".to_string(),
    );
    write_file(file1);
    run_bit_command(repository_dir.path(), &["add", "."])
        .assert()
        .success();
    bit_commit(repository_dir.path(), "Add test file")
        .assert()
        .success();

    let commit1_sha = get_head_commit_sha(repository_dir.path())?;

    // Second commit - modify the file
    let file1_modified = FileSpec::new(
        repository_dir.path().join("test.txt"),
        "modified content\n".to_string(),
    );
    write_file(file1_modified);

    run_bit_command(repository_dir.path(), &["add", "."])
        .assert()
        .success();
    bit_commit(repository_dir.path(), "Update test file")
        .assert()
        .success();

    let commit2_sha = get_head_commit_sha(repository_dir.path())?;

    // Run log with --patch and --oneline flags
    let output = run_bit_command(
        repository_dir.path(),
        &["log", "--patch", "--oneline", "--decorate=none"],
    )
    .assert()
    .success();

    let stdout = String::from_utf8(output.get_output().stdout.clone())?;

    // Verify oneline format for commits (abbreviated SHA + message)
    assert!(stdout.contains(&commit2_sha[..7]));
    assert!(stdout.contains(&commit1_sha[..7]));
    assert!(stdout.contains("Update test file"));
    assert!(stdout.contains("Add test file"));

    // Verify patches are still included despite oneline format
    assert!(stdout.contains("diff --git a/test.txt b/test.txt"));
    assert!(stdout.contains("-original content"));
    assert!(stdout.contains("+modified content"));

    // For the initial commit
    assert!(stdout.contains("+original content"));

    // Verify format: abbreviated commit line followed by patch
    let lines: Vec<&str> = stdout.lines().collect();

    // Find the second commit line (should be first in output)
    let commit2_line_idx = lines
        .iter()
        .position(|line| line.contains(&commit2_sha[..7]) && line.contains("Update test file"))
        .expect("Second commit line not found");

    // Verify that diff follows shortly after commit line
    let has_diff_after_commit2 = lines[commit2_line_idx..]
        .iter()
        .take(5)
        .any(|line| line.contains("diff --git"));

    assert!(
        has_diff_after_commit2,
        "Diff should appear after commit line"
    );

    Ok(())
}
