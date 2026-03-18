use crate::common::command::{bit_commit, bit_merge, repository_dir, run_bit_command};
use crate::common::file::{FileSpec, write_file};
use assert_fs::TempDir;
use rstest::rstest;
use std::fs;

/// Regression guard: merge must abort when the workspace has unstaged local edits
/// that would be overwritten by the incoming branch.
///
/// History:
///   A: file.txt = "original\n"   (master stays at A)
///   B (feature): file.txt = "feature change\n"
///
/// After committing A on master and B on feature, we manually write
/// "local edits\n" to file.txt WITHOUT staging it, then attempt the merge.
///
/// Expected: non-zero exit, error names file.txt, local edits preserved.
///
/// Note: this test is likely to pass immediately because StaleFile detection
/// is already present in the migration layer. It acts as a regression guard.
#[rstest]
fn merge_dirty_workspace_protection(
    repository_dir: TempDir,
) -> Result<(), Box<dyn std::error::Error>> {
    let dir = repository_dir;

    run_bit_command(dir.path(), &["init"]).assert().success();

    // Commit A on master
    write_file(FileSpec::new(
        dir.path().join("file.txt"),
        "original\n".into(),
    ));
    run_bit_command(dir.path(), &["add", "."])
        .assert()
        .success();
    bit_commit(dir.path(), "Commit A").assert().success();

    // Create feature branch at A
    run_bit_command(dir.path(), &["branch", "create", "feature"])
        .assert()
        .success();

    // Commit B on feature: modify file.txt
    run_bit_command(dir.path(), &["checkout", "feature"])
        .assert()
        .success();
    write_file(FileSpec::new(
        dir.path().join("file.txt"),
        "feature change\n".into(),
    ));
    run_bit_command(dir.path(), &["add", "."])
        .assert()
        .success();
    bit_commit(dir.path(), "Commit B - feature")
        .assert()
        .success();

    // Switch back to master (file.txt = "original\n" again in workspace)
    run_bit_command(dir.path(), &["checkout", "master"])
        .assert()
        .success();

    // Dirty the workspace WITHOUT staging
    fs::write(dir.path().join("file.txt"), "local edits\n")?;

    // Merge must abort to protect the local edits
    let output = bit_merge(dir.path(), "feature", "dirty workspace merge")
        .assert()
        .failure();
    let stderr = String::from_utf8(output.get_output().stderr.clone())?;

    assert!(
        stderr.contains("file.txt"),
        "Error output should name the file with local edits, got:\n{}",
        stderr
    );

    // Local edits must not be overwritten
    let content = fs::read_to_string(dir.path().join("file.txt"))?;
    assert_eq!(
        content, "local edits\n",
        "Local edits must be preserved when merge aborts"
    );

    Ok(())
}
