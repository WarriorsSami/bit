use crate::common::command::{bit_commit, bit_merge, repository_dir, run_bit_command};
use crate::common::file::{FileSpec, delete_path, write_file};
use assert_fs::TempDir;
use rstest::rstest;

/// `bit diff --theirs` on a modify/delete conflict silently produces no output
/// because stage 3 (theirs) is absent when the incoming branch deleted the file.
///
/// History:
///   A: f.txt = "original"
///   B (master):  f.txt = "modified"   ← stage 2 (ours) present
///   C (feature): f.txt deleted        ← stage 3 (theirs) absent
#[rstest]
fn missing_stage_silently_skipped(
    repository_dir: TempDir,
) -> Result<(), Box<dyn std::error::Error>> {
    let dir = repository_dir;

    run_bit_command(dir.path(), &["init"]).assert().success();

    write_file(FileSpec::new(dir.path().join("f.txt"), "original\n".into()));
    run_bit_command(dir.path(), &["add", "."])
        .assert()
        .success();
    bit_commit(dir.path(), "A").assert().success();

    run_bit_command(dir.path(), &["branch", "create", "feature"])
        .assert()
        .success();

    write_file(FileSpec::new(dir.path().join("f.txt"), "modified\n".into()));
    run_bit_command(dir.path(), &["add", "."])
        .assert()
        .success();
    bit_commit(dir.path(), "B").assert().success();

    run_bit_command(dir.path(), &["checkout", "feature"])
        .assert()
        .success();
    delete_path(&dir.path().join("f.txt"));
    run_bit_command(dir.path(), &["add", "."])
        .assert()
        .success();
    bit_commit(dir.path(), "C").assert().success();

    run_bit_command(dir.path(), &["checkout", "master"])
        .assert()
        .success();
    bit_merge(dir.path(), "feature", "conflict")
        .assert()
        .failure();

    let out = run_bit_command(dir.path(), &["diff", "--theirs"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let text = String::from_utf8(out)?;

    assert!(
        text.trim().is_empty(),
        "Expected no output when stage 3 is missing, got:\n{}",
        text
    );

    Ok(())
}
