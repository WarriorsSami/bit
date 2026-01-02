use crate::common::command::{bit_commit, get_head_commit_sha, run_bit_command};
use crate::common::file::{FileSpec, write_file};
use assert_fs::TempDir;
use rstest::rstest;

#[rstest]
fn show_log_from_head_commit(
    #[from(crate::common::command::repository_dir)] repository_dir: TempDir,
) -> Result<(), Box<dyn std::error::Error>> {
    // Initialize repository
    run_bit_command(repository_dir.path(), &["init"])
        .assert()
        .success();

    // Create a linear history with 3 commits
    for i in 1..=3 {
        let file = FileSpec::new(
            repository_dir.path().join(format!("file{}.txt", i)),
            format!("Content {}", i),
        );
        write_file(file);
        run_bit_command(repository_dir.path(), &["add", "."])
            .assert()
            .success();
        bit_commit(repository_dir.path(), &format!("Commit {}", i))
            .assert()
            .success();
    }

    // Get the HEAD commit SHA
    let head_sha = get_head_commit_sha(repository_dir.path())?;

    // Run the log command starting from HEAD using the SHA
    let output_from_sha = run_bit_command(
        repository_dir.path(),
        &["log", &head_sha, "--decorate=none"],
    )
    .assert()
    .success();

    let stdout_from_sha = String::from_utf8(output_from_sha.get_output().stdout.clone())?;

    // Run the log command without specifying a revision (defaults to HEAD)
    let output_default = run_bit_command(repository_dir.path(), &["log", "--decorate=none"])
        .assert()
        .success();

    let stdout_default = String::from_utf8(output_default.get_output().stdout.clone())?;

    // Extract commit SHAs from both outputs
    let shas_from_sha: Vec<&str> = stdout_from_sha
        .lines()
        .filter(|line| line.starts_with("commit "))
        .map(|line| line.strip_prefix("commit ").unwrap())
        .collect();

    let shas_default: Vec<&str> = stdout_default
        .lines()
        .filter(|line| line.starts_with("commit "))
        .map(|line| line.strip_prefix("commit ").unwrap())
        .collect();

    // Verify both outputs are identical
    assert_eq!(
        shas_from_sha.len(),
        shas_default.len(),
        "Both outputs should have the same number of commits"
    );

    assert_eq!(
        shas_from_sha, shas_default,
        "Log from HEAD SHA should match log without revision (default HEAD)"
    );

    // Verify we have all 3 commits
    assert_eq!(shas_from_sha.len(), 3, "Expected 3 commits in output");

    Ok(())
}
