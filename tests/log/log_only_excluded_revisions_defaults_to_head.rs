use crate::common::command::{bit_commit_with_timestamp, repository_dir, run_bit_command};
use crate::common::file::{FileSpec, write_file};
use assert_fs::TempDir;
use rstest::rstest;

#[rstest]
fn log_only_excluded_revisions_defaults_to_head(
    repository_dir: TempDir,
) -> Result<(), Box<dyn std::error::Error>> {
    // This test exercises the edge case where only excluded revisions are provided
    // (e.g., ^feature ^bugfix) without any explicit included revision.
    // In this case, the code should default to including HEAD.
    //
    // The history looks like this:
    //
    //   master (HEAD) (T6)
    //      |
    //   master (T5)
    //      |
    //   master (T4)
    //      |
    //   master (T3) -------- feature (T2)
    //      |
    //   bugfix (T1)
    //      |
    //   initial (T0)
    //
    // When we run: bit log ^bugfix ^feature
    // It should default to: bit log ^bugfix ^feature HEAD
    // Expected output: master (T6), master (T5), master (T4), master (T3)
    // Should NOT see: feature (T2), bugfix (T1), initial (T0)

    run_bit_command(repository_dir.path(), &["init"])
        .assert()
        .success();

    // Create initial commit (T0 = 2023-01-01 10:00:00)
    let file0 = FileSpec::new(
        repository_dir.path().join("file0.txt"),
        "initial".to_string(),
    );
    write_file(file0);
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

    // Create bugfix branch from initial commit
    run_bit_command(repository_dir.path(), &["branch", "create", "bugfix"])
        .assert()
        .success();
    run_bit_command(repository_dir.path(), &["checkout", "bugfix"])
        .assert()
        .success();

    // Add commit on bugfix branch (T1 = 2023-01-01 11:00:00)
    let file_b1 = FileSpec::new(
        repository_dir.path().join("bugfix1.txt"),
        "bugfix 1".to_string(),
    );
    write_file(file_b1);
    run_bit_command(repository_dir.path(), &["add", "."])
        .assert()
        .success();
    bit_commit_with_timestamp(
        repository_dir.path(),
        "Bugfix commit 1",
        "2023-01-01 11:00:00 +0000",
    )
    .assert()
    .success();

    // Switch back to master and continue development
    run_bit_command(repository_dir.path(), &["checkout", "master"])
        .assert()
        .success();

    // Add commit on master (T3 = 2023-01-01 13:00:00)
    let file_m1 = FileSpec::new(
        repository_dir.path().join("master1.txt"),
        "master 1".to_string(),
    );
    write_file(file_m1);
    run_bit_command(repository_dir.path(), &["add", "."])
        .assert()
        .success();
    bit_commit_with_timestamp(
        repository_dir.path(),
        "Master commit 1",
        "2023-01-01 13:00:00 +0000",
    )
    .assert()
    .success();

    // Create feature branch from master commit 1
    run_bit_command(repository_dir.path(), &["branch", "create", "feature"])
        .assert()
        .success();
    run_bit_command(repository_dir.path(), &["checkout", "feature"])
        .assert()
        .success();

    // Add commit on feature branch (T2 = 2023-01-01 12:00:00) - older timestamp
    let file_f1 = FileSpec::new(
        repository_dir.path().join("feature1.txt"),
        "feature 1".to_string(),
    );
    write_file(file_f1);
    run_bit_command(repository_dir.path(), &["add", "."])
        .assert()
        .success();
    bit_commit_with_timestamp(
        repository_dir.path(),
        "Feature commit 1",
        "2023-01-01 12:00:00 +0000",
    )
    .assert()
    .success();

    // Switch back to master and continue
    run_bit_command(repository_dir.path(), &["checkout", "master"])
        .assert()
        .success();

    // Add more commits on master (T4 = 2023-01-01 14:00:00)
    let file_m2 = FileSpec::new(
        repository_dir.path().join("master2.txt"),
        "master 2".to_string(),
    );
    write_file(file_m2);
    run_bit_command(repository_dir.path(), &["add", "."])
        .assert()
        .success();
    bit_commit_with_timestamp(
        repository_dir.path(),
        "Master commit 2",
        "2023-01-01 14:00:00 +0000",
    )
    .assert()
    .success();

    // Add another commit on master (T5 = 2023-01-01 15:00:00)
    let file_m3 = FileSpec::new(
        repository_dir.path().join("master3.txt"),
        "master 3".to_string(),
    );
    write_file(file_m3);
    run_bit_command(repository_dir.path(), &["add", "."])
        .assert()
        .success();
    bit_commit_with_timestamp(
        repository_dir.path(),
        "Master commit 3",
        "2023-01-01 15:00:00 +0000",
    )
    .assert()
    .success();

    // Add final commit on master (T6 = 2023-01-01 16:00:00)
    let file_m4 = FileSpec::new(
        repository_dir.path().join("master4.txt"),
        "master 4".to_string(),
    );
    write_file(file_m4);
    run_bit_command(repository_dir.path(), &["add", "."])
        .assert()
        .success();
    bit_commit_with_timestamp(
        repository_dir.path(),
        "Master commit 4",
        "2023-01-01 16:00:00 +0000",
    )
    .assert()
    .success();

    // Test with only excluded revisions: ^bugfix ^feature
    // This should default to including HEAD (master)
    // Expected: Master commits 4, 3, 2
    // Should NOT see: Master commit 1 (ancestor of feature), Feature commit 1, Bugfix commit 1, Initial commit
    let output = run_bit_command(
        repository_dir.path(),
        &["log", "^bugfix", "^feature", "--decorate=none"],
    )
    .assert()
    .success();

    let stdout = String::from_utf8(output.get_output().stdout.clone())?;

    // Should include commits on master after the feature branch point
    assert!(
        stdout.contains("    Master commit 4"),
        "Expected 'Master commit 4' when only excluded revisions provided"
    );
    assert!(
        stdout.contains("    Master commit 3"),
        "Expected 'Master commit 3' when only excluded revisions provided"
    );
    assert!(
        stdout.contains("    Master commit 2"),
        "Expected 'Master commit 2' when only excluded revisions provided"
    );

    // Should NOT include commits from excluded branches
    assert!(
        !stdout.contains("    Feature commit 1"),
        "Should not include 'Feature commit 1' when it's excluded"
    );
    assert!(
        !stdout.contains("    Bugfix commit 1"),
        "Should not include 'Bugfix commit 1' when it's excluded"
    );

    // Should NOT include common ancestors (including Master commit 1 which is ancestor of feature)
    assert!(
        !stdout.contains("    Master commit 1"),
        "Should not include 'Master commit 1' (ancestor of excluded feature branch)"
    );
    assert!(
        !stdout.contains("    Initial commit"),
        "Should not include 'Initial commit' (common ancestor)"
    );

    Ok(())
}
