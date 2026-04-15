use crate::common::command::{bit_commit, bit_merge, repository_dir, run_bit_command};
use crate::common::file::{FileSpec, delete_path, write_file};
use assert_fs::TempDir;
use pretty_assertions::assert_eq;
use rstest::rstest;

/// Porcelain status uses "DU" when we deleted a file that they modified.
///
/// History:
///   A: f.txt = "original\n"
///   B (master):  f.txt deleted           (we deleted)
///   C (topic):   f.txt = "modified\n"    (they modified)
///
/// After merging topic into master (fails), `status --porcelain` must output "DU f.txt".
#[rstest]
fn report_conflict_porcelain_deleted_by_us(
    repository_dir: TempDir,
) -> Result<(), Box<dyn std::error::Error>> {
    let dir = repository_dir;

    run_bit_command(dir.path(), &["init"]).assert().success();

    write_file(FileSpec::new(dir.path().join("f.txt"), "original\n".into()));
    run_bit_command(dir.path(), &["add", "."])
        .assert()
        .success();
    bit_commit(dir.path(), "Commit A").assert().success();

    run_bit_command(dir.path(), &["branch", "create", "topic"])
        .assert()
        .success();

    // Delete f.txt on master (we deleted)
    delete_path(&dir.path().join("f.txt"));
    run_bit_command(dir.path(), &["add", "."])
        .assert()
        .success();
    bit_commit(dir.path(), "Commit B - delete")
        .assert()
        .success();

    // Modify f.txt on topic (they modified)
    run_bit_command(dir.path(), &["checkout", "topic"])
        .assert()
        .success();
    write_file(FileSpec::new(dir.path().join("f.txt"), "modified\n".into()));
    run_bit_command(dir.path(), &["add", "."])
        .assert()
        .success();
    bit_commit(dir.path(), "Commit C - modify")
        .assert()
        .success();

    run_bit_command(dir.path(), &["checkout", "master"])
        .assert()
        .success();

    bit_merge(dir.path(), "topic", "merge topic")
        .assert()
        .failure();

    let output = run_bit_command(dir.path(), &["status", "--porcelain"])
        .assert()
        .success();
    let stdout = String::from_utf8(output.get_output().stdout.clone())?;

    assert_eq!(stdout, "DU f.txt\n");

    Ok(())
}
