use crate::common::command::{bit_commit_with_timestamp, repository_dir, run_bit_command};
use crate::common::file::{FileSpec, write_file};
use assert_fs::TempDir;
use rstest::rstest;

#[rstest]
fn log_multiple_branches_with_timestamp_ordering(
    repository_dir: TempDir,
) -> Result<(), Box<dyn std::error::Error>> {
    // Initialize repository
    run_bit_command(repository_dir.path(), &["init"])
        .assert()
        .success();

    // Create initial commit (T0 = 2023-01-01 10:00:00)
    let file1 = FileSpec::new(
        repository_dir.path().join("file1.txt"),
        "initial content".to_string(),
    );
    write_file(file1);
    run_bit_command(repository_dir.path(), &["add", "."])
        .assert()
        .success();
    bit_commit_with_timestamp(
        repository_dir.path(),
        "Initial commit",
        "2023-01-01 10:00:00 +0000",
    )
    .assert()
    .success();

    // Create branch A and add commits with timestamps T1 and T3
    run_bit_command(repository_dir.path(), &["branch", "create", "branch-a"])
        .assert()
        .success();
    run_bit_command(repository_dir.path(), &["checkout", "branch-a"])
        .assert()
        .success();

    // Commit A1 (T1 = 2023-01-01 11:00:00)
    let file_a1 = FileSpec::new(
        repository_dir.path().join("file_a1.txt"),
        "branch a - commit 1".to_string(),
    );
    write_file(file_a1);
    run_bit_command(repository_dir.path(), &["add", "."])
        .assert()
        .success();
    bit_commit_with_timestamp(
        repository_dir.path(),
        "Branch A - Commit 1",
        "2023-01-01 11:00:00 +0000",
    )
    .assert()
    .success();

    // Commit A2 (T3 = 2023-01-01 13:00:00)
    let file_a2 = FileSpec::new(
        repository_dir.path().join("file_a2.txt"),
        "branch a - commit 2".to_string(),
    );
    write_file(file_a2);
    run_bit_command(repository_dir.path(), &["add", "."])
        .assert()
        .success();
    bit_commit_with_timestamp(
        repository_dir.path(),
        "Branch A - Commit 2",
        "2023-01-01 13:00:00 +0000",
    )
    .assert()
    .success();

    // Switch to main and create branch B with timestamps T2 and T4
    run_bit_command(repository_dir.path(), &["checkout", "master"])
        .assert()
        .success();
    run_bit_command(repository_dir.path(), &["branch", "create", "branch-b"])
        .assert()
        .success();
    run_bit_command(repository_dir.path(), &["checkout", "branch-b"])
        .assert()
        .success();

    // Commit B1 (T2 = 2023-01-01 12:00:00)
    let file_b1 = FileSpec::new(
        repository_dir.path().join("file_b1.txt"),
        "branch b - commit 1".to_string(),
    );
    write_file(file_b1);
    run_bit_command(repository_dir.path(), &["add", "."])
        .assert()
        .success();
    bit_commit_with_timestamp(
        repository_dir.path(),
        "Branch B - Commit 1",
        "2023-01-01 12:00:00 +0000",
    )
    .assert()
    .success();

    // Commit B2 (T4 = 2023-01-01 14:00:00)
    let file_b2 = FileSpec::new(
        repository_dir.path().join("file_b2.txt"),
        "branch b - commit 2".to_string(),
    );
    write_file(file_b2);
    run_bit_command(repository_dir.path(), &["add", "."])
        .assert()
        .success();
    bit_commit_with_timestamp(
        repository_dir.path(),
        "Branch B - Commit 2",
        "2023-01-01 14:00:00 +0000",
    )
    .assert()
    .success();

    // Run log with multiple revisions: branch-a and branch-b
    // Expected order based on timestamps (newest first):
    // T4: Branch B - Commit 2 (14:00)
    // T3: Branch A - Commit 2 (13:00)
    // T2: Branch B - Commit 1 (12:00)
    // T1: Branch A - Commit 1 (11:00)
    // T0: Initial commit (10:00)
    let output = run_bit_command(
        repository_dir.path(),
        &["log", "branch-a", "branch-b", "--decorate=none"],
    )
    .assert()
    .success();

    let stdout = String::from_utf8(output.get_output().stdout.clone())?;

    // Verify all 5 commits appear
    assert!(
        stdout.contains("    Branch B - Commit 2"),
        "Expected 'Branch B - Commit 2' in output"
    );
    assert!(
        stdout.contains("    Branch A - Commit 2"),
        "Expected 'Branch A - Commit 2' in output"
    );
    assert!(
        stdout.contains("    Branch B - Commit 1"),
        "Expected 'Branch B - Commit 1' in output"
    );
    assert!(
        stdout.contains("    Branch A - Commit 1"),
        "Expected 'Branch A - Commit 1' in output"
    );
    assert!(
        stdout.contains("    Initial commit"),
        "Expected 'Initial commit' in output"
    );

    // Verify timestamp-based ordering (partial order)
    let b2_pos = stdout.find("    Branch B - Commit 2").unwrap();
    let a2_pos = stdout.find("    Branch A - Commit 2").unwrap();
    let b1_pos = stdout.find("    Branch B - Commit 1").unwrap();
    let a1_pos = stdout.find("    Branch A - Commit 1").unwrap();
    let initial_pos = stdout.find("    Initial commit").unwrap();

    assert!(
        b2_pos < a2_pos,
        "Branch B - Commit 2 (14:00) should appear before Branch A - Commit 2 (13:00)"
    );
    assert!(
        a2_pos < b1_pos,
        "Branch A - Commit 2 (13:00) should appear before Branch B - Commit 1 (12:00)"
    );
    assert!(
        b1_pos < a1_pos,
        "Branch B - Commit 1 (12:00) should appear before Branch A - Commit 1 (11:00)"
    );
    assert!(
        a1_pos < initial_pos,
        "Branch A - Commit 1 (11:00) should appear before Initial commit (10:00)"
    );

    Ok(())
}
