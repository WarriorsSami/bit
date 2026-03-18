use crate::common::command::{bit_commit, bit_merge, repository_dir, run_bit_command};
use crate::common::file::{FileSpec, write_file};
use assert_fs::TempDir;
use rstest::rstest;
use std::fs;

/// Test that a file/directory path collision is detected and handled gracefully
///
/// History:
///   A: only anchor.txt
///   B (master):  f.txt as a regular file ("file content\n")
///   C (feature): f.txt/g.txt — a *directory* named f.txt containing g.txt
///
/// Expected: non-zero exit; master's f.txt renamed to f.txt~HEAD (Git convention);
/// feature's f.txt/g.txt present in workspace.
///
/// Note: if the implementation uses the branch name instead (e.g., f.txt~master),
/// update the assertion and the comment below accordingly.
#[rstest]
fn merge_file_directory_conflict(
    repository_dir: TempDir,
) -> Result<(), Box<dyn std::error::Error>> {
    let dir = repository_dir;

    run_bit_command(dir.path(), &["init"]).assert().success();

    // Commit A: anchor only
    write_file(FileSpec::new(
        dir.path().join("anchor.txt"),
        "anchor\n".into(),
    ));
    run_bit_command(dir.path(), &["add", "."])
        .assert()
        .success();
    bit_commit(dir.path(), "Commit A").assert().success();

    // Create feature branch at A
    run_bit_command(dir.path(), &["branch", "create", "feature"])
        .assert()
        .success();

    // Commit B on master: add f.txt as a regular file
    write_file(FileSpec::new(
        dir.path().join("f.txt"),
        "file content\n".into(),
    ));
    run_bit_command(dir.path(), &["add", "."])
        .assert()
        .success();
    bit_commit(dir.path(), "Commit B - f.txt as file")
        .assert()
        .success();

    // Commit C on feature: add f.txt/g.txt (directory with same name)
    // write_file calls create_dir_all so it naturally creates the directory
    run_bit_command(dir.path(), &["checkout", "feature"])
        .assert()
        .success();
    write_file(FileSpec::new(
        dir.path().join("f.txt").join("g.txt"),
        "dir content\n".into(),
    ));
    run_bit_command(dir.path(), &["add", "."])
        .assert()
        .success();
    bit_commit(dir.path(), "Commit C - f.txt as directory")
        .assert()
        .success();

    run_bit_command(dir.path(), &["checkout", "master"])
        .assert()
        .success();

    // Merge must fail due to path-type collision
    bit_merge(dir.path(), "feature", "file-dir conflict")
        .assert()
        .failure();

    // f.txt must now be a directory (from feature)
    assert!(
        dir.path().join("f.txt").is_dir(),
        "f.txt should be a directory after resolving file/directory collision"
    );

    // feature's nested file must be accessible
    assert!(
        dir.path().join("f.txt").join("g.txt").exists(),
        "f.txt/g.txt should be present in workspace"
    );

    // master's original file must be renamed with ~HEAD suffix to avoid loss
    // Git uses ~HEAD; if the implementation chooses a branch name (e.g., ~master),
    // change this assertion to check for "f.txt~master" instead.
    let renamed = dir.path().join("f.txt~HEAD");
    assert!(
        renamed.exists(),
        "master's f.txt should be renamed to f.txt~HEAD, but it was not found"
    );
    let renamed_content = fs::read_to_string(&renamed)?;
    assert_eq!(
        renamed_content, "file content\n",
        "f.txt~HEAD should contain master's original file content"
    );

    Ok(())
}
