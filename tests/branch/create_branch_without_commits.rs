use crate::common::command::{repository_dir, run_bit_command};
use assert_fs::TempDir;
use predicates::prelude::*;
use rstest::rstest;

#[rstest]
fn create_branch_without_commits(
    repository_dir: TempDir,
) -> Result<(), Box<dyn std::error::Error>> {
    let repository_dir = repository_dir;

    // initialize the repository but don't make any commits
    run_bit_command(repository_dir.path(), &["init"])
        .assert()
        .success();

    // attempt to create a branch without any commits
    let branch_name = "feature";
    run_bit_command(repository_dir.path(), &["branch", branch_name])
        .assert()
        .failure()
        .stderr(predicate::str::contains("failed to read HEAD"));

    // assert the branch ref does NOT exist
    let branch_ref_path = repository_dir
        .path()
        .join(".git")
        .join("refs")
        .join("heads")
        .join(branch_name);
    assert!(!branch_ref_path.exists());

    Ok(())
}
