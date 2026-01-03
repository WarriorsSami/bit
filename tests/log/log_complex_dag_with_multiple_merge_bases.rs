use crate::common::command::{bit_commit_with_timestamp, repository_dir, run_bit_command};
use crate::common::file::{FileSpec, write_file};
use assert_fs::TempDir;
use rstest::rstest;

#[rstest]
fn log_complex_dag_with_multiple_merge_bases(
    repository_dir: TempDir,
) -> Result<(), Box<dyn std::error::Error>> {
    // Create a complex DAG structure with multiple paths and shared history
    run_bit_command(repository_dir.path(), &["init"])
        .assert()
        .success();

    // Common base
    let file1 = FileSpec::new(repository_dir.path().join("base.txt"), "base".to_string());
    write_file(file1);
    run_bit_command(repository_dir.path(), &["add", "."])
        .assert()
        .success();
    bit_commit_with_timestamp(repository_dir.path(), "Base", "2023-01-01 10:00:00 +0000")
        .assert()
        .success();

    // Create branch structure:
    //        Base (T0)
    //       /    \
    //      /      \
    //   Dev-1(T1) Exp-1(T2)
    //     |         |
    //   Dev-2(T3) Exp-2(T4)
    //     |         |
    //   Dev-3(T5) Exp-3(T6)

    // Dev branch
    run_bit_command(repository_dir.path(), &["branch", "create", "dev"])
        .assert()
        .success();
    run_bit_command(repository_dir.path(), &["checkout", "dev"])
        .assert()
        .success();

    for i in 1..=3 {
        let file = FileSpec::new(
            repository_dir.path().join(format!("dev{}.txt", i)),
            format!("dev {}", i),
        );
        write_file(file);
        run_bit_command(repository_dir.path(), &["add", "."])
            .assert()
            .success();
        let timestamp = format!("2023-01-01 {}:00:00 +0000", 10 + (i * 2 - 1));
        bit_commit_with_timestamp(repository_dir.path(), &format!("Dev-{}", i), &timestamp)
            .assert()
            .success();
    }

    // Experimental branch
    run_bit_command(repository_dir.path(), &["checkout", "master"])
        .assert()
        .success();
    run_bit_command(repository_dir.path(), &["branch", "create", "experimental"])
        .assert()
        .success();
    run_bit_command(repository_dir.path(), &["checkout", "experimental"])
        .assert()
        .success();

    for i in 1..=3 {
        let file = FileSpec::new(
            repository_dir.path().join(format!("exp{}.txt", i)),
            format!("exp {}", i),
        );
        write_file(file);
        run_bit_command(repository_dir.path(), &["add", "."])
            .assert()
            .success();
        let timestamp = format!("2023-01-01 {}:00:00 +0000", 10 + (i * 2));
        bit_commit_with_timestamp(repository_dir.path(), &format!("Exp-{}", i), &timestamp)
            .assert()
            .success();
    }

    // Log both branches
    // Expected order (by timestamp):
    // Exp-3 (16:00)
    // Dev-3 (15:00)
    // Exp-2 (14:00)
    // Dev-2 (13:00)
    // Exp-1 (12:00)
    // Dev-1 (11:00)
    // Base (10:00)
    let output = run_bit_command(
        repository_dir.path(),
        &["log", "dev", "experimental", "--decorate=none"],
    )
    .assert()
    .success();

    let stdout = String::from_utf8(output.get_output().stdout.clone())?;

    // Extract positions
    let positions = [
        ("Exp-3", stdout.find("    Exp-3").unwrap()),
        ("Dev-3", stdout.find("    Dev-3").unwrap()),
        ("Exp-2", stdout.find("    Exp-2").unwrap()),
        ("Dev-2", stdout.find("    Dev-2").unwrap()),
        ("Exp-1", stdout.find("    Exp-1").unwrap()),
        ("Dev-1", stdout.find("    Dev-1").unwrap()),
        ("Base", stdout.find("    Base").unwrap()),
    ];

    // Verify strict ordering
    for i in 0..positions.len() - 1 {
        assert!(
            positions[i].1 < positions[i + 1].1,
            "{} should appear before {}",
            positions[i].0,
            positions[i + 1].0
        );
    }

    // Base should appear only once
    let base_count = stdout.matches("    Base").count();
    assert_eq!(base_count, 1, "Base should appear exactly once");

    // Total commits should be 7
    let commit_count = stdout.matches("commit ").count();
    assert_eq!(commit_count, 7, "Expected 7 commits");

    Ok(())
}
