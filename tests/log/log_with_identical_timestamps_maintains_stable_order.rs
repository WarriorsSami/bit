use crate::common::command::{bit_commit_with_timestamp, repository_dir, run_bit_command};
use crate::common::file::{FileSpec, write_file};
use assert_fs::TempDir;
use rstest::rstest;

#[rstest]
fn log_with_identical_timestamps_maintains_stable_order(
    repository_dir: TempDir,
) -> Result<(), Box<dyn std::error::Error>> {
    // Test that when multiple commits have identical timestamps,
    // the implementation maintains a stable, predictable order
    run_bit_command(repository_dir.path(), &["init"])
        .assert()
        .success();

    // Create base commit
    let file_base = FileSpec::new(repository_dir.path().join("base.txt"), "base".to_string());
    write_file(file_base);
    run_bit_command(repository_dir.path(), &["add", "."])
        .assert()
        .success();
    bit_commit_with_timestamp(repository_dir.path(), "Base", "2023-01-01 10:00:00 +0000")
        .assert()
        .success();

    // Create branch A with two commits at the same timestamp
    run_bit_command(repository_dir.path(), &["branch", "create", "branch-a"])
        .assert()
        .success();
    run_bit_command(repository_dir.path(), &["checkout", "branch-a"])
        .assert()
        .success();

    let file_a1 = FileSpec::new(repository_dir.path().join("a1.txt"), "a1".to_string());
    write_file(file_a1);
    run_bit_command(repository_dir.path(), &["add", "."])
        .assert()
        .success();
    bit_commit_with_timestamp(
        repository_dir.path(),
        "A-Same-Time-1",
        "2023-01-01 12:00:00 +0000",
    )
    .assert()
    .success();

    let file_a2 = FileSpec::new(repository_dir.path().join("a2.txt"), "a2".to_string());
    write_file(file_a2);
    run_bit_command(repository_dir.path(), &["add", "."])
        .assert()
        .success();
    bit_commit_with_timestamp(
        repository_dir.path(),
        "A-Same-Time-2",
        "2023-01-01 12:00:00 +0000",
    )
    .assert()
    .success();

    // Create branch B with two commits at the same timestamp
    run_bit_command(repository_dir.path(), &["checkout", "master"])
        .assert()
        .success();
    run_bit_command(repository_dir.path(), &["branch", "create", "branch-b"])
        .assert()
        .success();
    run_bit_command(repository_dir.path(), &["checkout", "branch-b"])
        .assert()
        .success();

    let file_b1 = FileSpec::new(repository_dir.path().join("b1.txt"), "b1".to_string());
    write_file(file_b1);
    run_bit_command(repository_dir.path(), &["add", "."])
        .assert()
        .success();
    bit_commit_with_timestamp(
        repository_dir.path(),
        "B-Same-Time-1",
        "2023-01-01 12:00:00 +0000",
    )
    .assert()
    .success();

    let file_b2 = FileSpec::new(repository_dir.path().join("b2.txt"), "b2".to_string());
    write_file(file_b2);
    run_bit_command(repository_dir.path(), &["add", "."])
        .assert()
        .success();
    bit_commit_with_timestamp(
        repository_dir.path(),
        "B-Same-Time-2",
        "2023-01-01 12:00:00 +0000",
    )
    .assert()
    .success();

    // Log both branches
    let output = run_bit_command(
        repository_dir.path(),
        &["log", "branch-a", "branch-b", "--decorate=none"],
    )
    .assert()
    .success();

    let stdout = String::from_utf8(output.get_output().stdout.clone())?;

    // All commits should be present
    assert!(stdout.contains("    A-Same-Time-1"));
    assert!(stdout.contains("    A-Same-Time-2"));
    assert!(stdout.contains("    B-Same-Time-1"));
    assert!(stdout.contains("    B-Same-Time-2"));
    assert!(stdout.contains("    Base"));

    // All same-timestamp commits should appear before Base
    let base_pos = stdout.find("    Base").unwrap();
    let a1_pos = stdout.find("    A-Same-Time-1").unwrap();
    let a2_pos = stdout.find("    A-Same-Time-2").unwrap();
    let b1_pos = stdout.find("    B-Same-Time-1").unwrap();
    let b2_pos = stdout.find("    B-Same-Time-2").unwrap();

    assert!(a1_pos < base_pos);
    assert!(a2_pos < base_pos);
    assert!(b1_pos < base_pos);
    assert!(b2_pos < base_pos);

    // Verify total commit count
    let commit_count = stdout.matches("commit ").count();
    assert_eq!(commit_count, 5, "Expected 5 commits in output");

    Ok(())
}
