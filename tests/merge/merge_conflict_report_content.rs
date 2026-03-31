use crate::common::command::{bit_commit, bit_merge, repository_dir, run_bit_command};
use crate::common::file::{FileSpec, write_file};
use assert_fs::TempDir;
use rstest::rstest;

/// Test that a content conflict emits "Auto-merging" and "CONFLICT (content)" progress lines
///
/// History:
///   A: file.txt = "base\nshared\nmore\n"
///   B (master):  "shared" → "ours change"
///   C (feature): "shared" → "theirs change"
///
/// Expected stdout:
///   Auto-merging file.txt
///   CONFLICT (content): Merge conflict in file.txt
#[rstest]
fn merge_conflict_report_content(
    repository_dir: TempDir,
) -> Result<(), Box<dyn std::error::Error>> {
    let dir = repository_dir;

    run_bit_command(dir.path(), &["init"]).assert().success();

    // Commit A: base content
    write_file(FileSpec::new(
        dir.path().join("file.txt"),
        "base\nshared\nmore\n".into(),
    ));
    run_bit_command(dir.path(), &["add", "."])
        .assert()
        .success();
    bit_commit(dir.path(), "Commit A").assert().success();

    // Create feature branch at A
    run_bit_command(dir.path(), &["branch", "create", "feature"])
        .assert()
        .success();

    // Commit B on master: modify the shared line
    write_file(FileSpec::new(
        dir.path().join("file.txt"),
        "base\nours change\nmore\n".into(),
    ));
    run_bit_command(dir.path(), &["add", "."])
        .assert()
        .success();
    bit_commit(dir.path(), "Commit B - ours").assert().success();

    // Commit C on feature: modify the same shared line differently
    run_bit_command(dir.path(), &["checkout", "feature"])
        .assert()
        .success();
    write_file(FileSpec::new(
        dir.path().join("file.txt"),
        "base\ntheirs change\nmore\n".into(),
    ));
    run_bit_command(dir.path(), &["add", "."])
        .assert()
        .success();
    bit_commit(dir.path(), "Commit C - theirs")
        .assert()
        .success();

    run_bit_command(dir.path(), &["checkout", "master"])
        .assert()
        .success();

    // Merge must fail with a conflict
    let output = bit_merge(dir.path(), "feature", "conflict merge")
        .assert()
        .failure();
    let stdout = String::from_utf8(output.get_output().stdout.clone())?;

    assert!(
        stdout.contains("Auto-merging file.txt"),
        "Expected 'Auto-merging file.txt' in stdout, got:\n{}",
        stdout
    );
    assert!(
        stdout.contains("CONFLICT (content): Merge conflict in file.txt"),
        "Expected 'CONFLICT (content): Merge conflict in file.txt' in stdout, got:\n{}",
        stdout
    );

    Ok(())
}
