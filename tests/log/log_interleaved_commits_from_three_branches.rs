use crate::common::command::{bit_commit_with_timestamp, repository_dir, run_bit_command};
use crate::common::file::{FileSpec, write_file};
use assert_fs::TempDir;
use rstest::rstest;

#[rstest]
fn log_interleaved_commits_from_three_branches(
    repository_dir: TempDir,
) -> Result<(), Box<dyn std::error::Error>> {
    // This test creates a complex history with three branches where commits are interleaved
    // by timestamp across all branches, ensuring proper partial ordering
    run_bit_command(repository_dir.path(), &["init"])
        .assert()
        .success();

    // Create initial commit
    let file1 = FileSpec::new(
        repository_dir.path().join("base.txt"),
        "base content".to_string(),
    );
    write_file(file1);
    run_bit_command(repository_dir.path(), &["add", "."])
        .assert()
        .success();
    bit_commit_with_timestamp(repository_dir.path(), "Base", "2023-01-01 10:00:00 +0000")
        .assert()
        .success();

    // Create three branches from base
    for branch in &["alpha", "beta", "gamma"] {
        run_bit_command(repository_dir.path(), &["branch", "create", branch])
            .assert()
            .success();
    }

    // Alpha branch: commits at T1, T4, T7
    run_bit_command(repository_dir.path(), &["checkout", "alpha"])
        .assert()
        .success();

    let file_a1 = FileSpec::new(
        repository_dir.path().join("alpha1.txt"),
        "alpha 1".to_string(),
    );
    write_file(file_a1);
    run_bit_command(repository_dir.path(), &["add", "."])
        .assert()
        .success();
    bit_commit_with_timestamp(
        repository_dir.path(),
        "Alpha-1",
        "2023-01-01 11:00:00 +0000",
    )
    .assert()
    .success();

    let file_a2 = FileSpec::new(
        repository_dir.path().join("alpha2.txt"),
        "alpha 2".to_string(),
    );
    write_file(file_a2);
    run_bit_command(repository_dir.path(), &["add", "."])
        .assert()
        .success();
    bit_commit_with_timestamp(
        repository_dir.path(),
        "Alpha-2",
        "2023-01-01 14:00:00 +0000",
    )
    .assert()
    .success();

    let file_a3 = FileSpec::new(
        repository_dir.path().join("alpha3.txt"),
        "alpha 3".to_string(),
    );
    write_file(file_a3);
    run_bit_command(repository_dir.path(), &["add", "."])
        .assert()
        .success();
    bit_commit_with_timestamp(
        repository_dir.path(),
        "Alpha-3",
        "2023-01-01 17:00:00 +0000",
    )
    .assert()
    .success();

    // Beta branch: commits at T2, T5, T8
    run_bit_command(repository_dir.path(), &["checkout", "beta"])
        .assert()
        .success();

    let file_b1 = FileSpec::new(
        repository_dir.path().join("beta1.txt"),
        "beta 1".to_string(),
    );
    write_file(file_b1);
    run_bit_command(repository_dir.path(), &["add", "."])
        .assert()
        .success();
    bit_commit_with_timestamp(repository_dir.path(), "Beta-1", "2023-01-01 12:00:00 +0000")
        .assert()
        .success();

    let file_b2 = FileSpec::new(
        repository_dir.path().join("beta2.txt"),
        "beta 2".to_string(),
    );
    write_file(file_b2);
    run_bit_command(repository_dir.path(), &["add", "."])
        .assert()
        .success();
    bit_commit_with_timestamp(repository_dir.path(), "Beta-2", "2023-01-01 15:00:00 +0000")
        .assert()
        .success();

    let file_b3 = FileSpec::new(
        repository_dir.path().join("beta3.txt"),
        "beta 3".to_string(),
    );
    write_file(file_b3);
    run_bit_command(repository_dir.path(), &["add", "."])
        .assert()
        .success();
    bit_commit_with_timestamp(repository_dir.path(), "Beta-3", "2023-01-01 18:00:00 +0000")
        .assert()
        .success();

    // Gamma branch: commits at T3, T6, T9
    run_bit_command(repository_dir.path(), &["checkout", "gamma"])
        .assert()
        .success();

    let file_g1 = FileSpec::new(
        repository_dir.path().join("gamma1.txt"),
        "gamma 1".to_string(),
    );
    write_file(file_g1);
    run_bit_command(repository_dir.path(), &["add", "."])
        .assert()
        .success();
    bit_commit_with_timestamp(
        repository_dir.path(),
        "Gamma-1",
        "2023-01-01 13:00:00 +0000",
    )
    .assert()
    .success();

    let file_g2 = FileSpec::new(
        repository_dir.path().join("gamma2.txt"),
        "gamma 2".to_string(),
    );
    write_file(file_g2);
    run_bit_command(repository_dir.path(), &["add", "."])
        .assert()
        .success();
    bit_commit_with_timestamp(
        repository_dir.path(),
        "Gamma-2",
        "2023-01-01 16:00:00 +0000",
    )
    .assert()
    .success();

    let file_g3 = FileSpec::new(
        repository_dir.path().join("gamma3.txt"),
        "gamma 3".to_string(),
    );
    write_file(file_g3);
    run_bit_command(repository_dir.path(), &["add", "."])
        .assert()
        .success();
    bit_commit_with_timestamp(
        repository_dir.path(),
        "Gamma-3",
        "2023-01-01 19:00:00 +0000",
    )
    .assert()
    .success();

    // Run log with all three branches
    // Expected order (newest first):
    // T9: Gamma-3 (19:00)
    // T8: Beta-3 (18:00)
    // T7: Alpha-3 (17:00)
    // T6: Gamma-2 (16:00)
    // T5: Beta-2 (15:00)
    // T4: Alpha-2 (14:00)
    // T3: Gamma-1 (13:00)
    // T2: Beta-1 (12:00)
    // T1: Alpha-1 (11:00)
    // T0: Base (10:00)
    let output = run_bit_command(
        repository_dir.path(),
        &["log", "alpha", "beta", "gamma", "--decorate=none"],
    )
    .assert()
    .success();

    let stdout = String::from_utf8(output.get_output().stdout.clone())?;

    // Extract commit positions
    let positions = vec![
        ("Gamma-3", stdout.find("    Gamma-3").unwrap()),
        ("Beta-3", stdout.find("    Beta-3").unwrap()),
        ("Alpha-3", stdout.find("    Alpha-3").unwrap()),
        ("Gamma-2", stdout.find("    Gamma-2").unwrap()),
        ("Beta-2", stdout.find("    Beta-2").unwrap()),
        ("Alpha-2", stdout.find("    Alpha-2").unwrap()),
        ("Gamma-1", stdout.find("    Gamma-1").unwrap()),
        ("Beta-1", stdout.find("    Beta-1").unwrap()),
        ("Alpha-1", stdout.find("    Alpha-1").unwrap()),
        ("Base", stdout.find("    Base").unwrap()),
    ];

    // Verify strict ordering based on timestamps
    for i in 0..positions.len() - 1 {
        assert!(
            positions[i].1 < positions[i + 1].1,
            "{} (pos {}) should appear before {} (pos {})",
            positions[i].0,
            positions[i].1,
            positions[i + 1].0,
            positions[i + 1].1
        );
    }

    // Verify all commits are present
    let commit_count = stdout.matches("commit ").count();
    assert_eq!(
        commit_count, 10,
        "Expected 10 commits in output, found {}",
        commit_count
    );

    Ok(())
}
