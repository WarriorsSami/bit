use crate::common::command::{bit_commit_with_timestamp, repository_dir, run_bit_command};
use crate::common::file::{FileSpec, write_file};
use assert_fs::TempDir;
use rstest::rstest;

#[rstest]
fn log_divergent_histories_with_common_ancestor(
    repository_dir: TempDir,
) -> Result<(), Box<dyn std::error::Error>> {
    // Test scenario where two branches diverge from a common ancestor
    // and we log both branches together
    run_bit_command(repository_dir.path(), &["init"])
        .assert()
        .success();

    // Create common ancestor commits
    let file1 = FileSpec::new(
        repository_dir.path().join("ancestor1.txt"),
        "ancestor 1".to_string(),
    );
    write_file(file1);
    run_bit_command(repository_dir.path(), &["add", "."])
        .assert()
        .success();
    bit_commit_with_timestamp(
        repository_dir.path(),
        "Ancestor-1",
        "2023-01-01 10:00:00 +0000",
    )
    .assert()
    .success();

    let file2 = FileSpec::new(
        repository_dir.path().join("ancestor2.txt"),
        "ancestor 2".to_string(),
    );
    write_file(file2);
    run_bit_command(repository_dir.path(), &["add", "."])
        .assert()
        .success();
    bit_commit_with_timestamp(
        repository_dir.path(),
        "Ancestor-2",
        "2023-01-01 11:00:00 +0000",
    )
    .assert()
    .success();

    // Create first diverging branch
    run_bit_command(repository_dir.path(), &["branch", "create", "feature-x"])
        .assert()
        .success();
    run_bit_command(repository_dir.path(), &["checkout", "feature-x"])
        .assert()
        .success();

    let file_x1 = FileSpec::new(
        repository_dir.path().join("feature_x1.txt"),
        "feature x 1".to_string(),
    );
    write_file(file_x1);
    run_bit_command(repository_dir.path(), &["add", "."])
        .assert()
        .success();
    bit_commit_with_timestamp(
        repository_dir.path(),
        "Feature-X-1",
        "2023-01-01 12:00:00 +0000",
    )
    .assert()
    .success();

    let file_x2 = FileSpec::new(
        repository_dir.path().join("feature_x2.txt"),
        "feature x 2".to_string(),
    );
    write_file(file_x2);
    run_bit_command(repository_dir.path(), &["add", "."])
        .assert()
        .success();
    bit_commit_with_timestamp(
        repository_dir.path(),
        "Feature-X-2",
        "2023-01-01 14:00:00 +0000",
    )
    .assert()
    .success();

    // Create second diverging branch from the same ancestor
    run_bit_command(repository_dir.path(), &["checkout", "master"])
        .assert()
        .success();
    run_bit_command(repository_dir.path(), &["branch", "create", "feature-y"])
        .assert()
        .success();
    run_bit_command(repository_dir.path(), &["checkout", "feature-y"])
        .assert()
        .success();

    let file_y1 = FileSpec::new(
        repository_dir.path().join("feature_y1.txt"),
        "feature y 1".to_string(),
    );
    write_file(file_y1);
    run_bit_command(repository_dir.path(), &["add", "."])
        .assert()
        .success();
    bit_commit_with_timestamp(
        repository_dir.path(),
        "Feature-Y-1",
        "2023-01-01 13:00:00 +0000",
    )
    .assert()
    .success();

    let file_y2 = FileSpec::new(
        repository_dir.path().join("feature_y2.txt"),
        "feature y 2".to_string(),
    );
    write_file(file_y2);
    run_bit_command(repository_dir.path(), &["add", "."])
        .assert()
        .success();
    bit_commit_with_timestamp(
        repository_dir.path(),
        "Feature-Y-2",
        "2023-01-01 15:00:00 +0000",
    )
    .assert()
    .success();

    // Log both feature branches
    // Expected order (by timestamp):
    // Feature-Y-2 (15:00)
    // Feature-X-2 (14:00)
    // Feature-Y-1 (13:00)
    // Feature-X-1 (12:00)
    // Ancestor-2 (11:00) - common ancestor
    // Ancestor-1 (10:00) - common ancestor
    let output = run_bit_command(
        repository_dir.path(),
        &["log", "feature-x", "feature-y", "--decorate=none"],
    )
    .assert()
    .success();

    let stdout = String::from_utf8(output.get_output().stdout.clone())?;

    // Verify all commits appear
    assert!(stdout.contains("    Feature-Y-2"));
    assert!(stdout.contains("    Feature-X-2"));
    assert!(stdout.contains("    Feature-Y-1"));
    assert!(stdout.contains("    Feature-X-1"));
    assert!(stdout.contains("    Ancestor-2"));
    assert!(stdout.contains("    Ancestor-1"));

    // Get positions
    let y2_pos = stdout.find("    Feature-Y-2").unwrap();
    let x2_pos = stdout.find("    Feature-X-2").unwrap();
    let y1_pos = stdout.find("    Feature-Y-1").unwrap();
    let x1_pos = stdout.find("    Feature-X-1").unwrap();
    let a2_pos = stdout.find("    Ancestor-2").unwrap();
    let a1_pos = stdout.find("    Ancestor-1").unwrap();

    // Verify timestamp ordering
    assert!(
        y2_pos < x2_pos,
        "Feature-Y-2 (15:00) should appear before Feature-X-2 (14:00)"
    );
    assert!(
        x2_pos < y1_pos,
        "Feature-X-2 (14:00) should appear before Feature-Y-1 (13:00)"
    );
    assert!(
        y1_pos < x1_pos,
        "Feature-Y-1 (13:00) should appear before Feature-X-1 (12:00)"
    );
    assert!(
        x1_pos < a2_pos,
        "Feature-X-1 (12:00) should appear before Ancestor-2 (11:00)"
    );
    assert!(
        a2_pos < a1_pos,
        "Ancestor-2 (11:00) should appear before Ancestor-1 (10:00)"
    );

    // Common ancestor should only appear once (not duplicated)
    let ancestor1_count = stdout.matches("    Ancestor-1").count();
    let ancestor2_count = stdout.matches("    Ancestor-2").count();
    assert_eq!(
        ancestor1_count, 1,
        "Ancestor-1 should appear exactly once, found {} times",
        ancestor1_count
    );
    assert_eq!(
        ancestor2_count, 1,
        "Ancestor-2 should appear exactly once, found {} times",
        ancestor2_count
    );

    Ok(())
}
