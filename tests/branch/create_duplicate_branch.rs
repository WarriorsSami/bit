use crate::common::command::{init_repository_dir, run_bit_command};
use assert_fs::TempDir;
use predicates::prelude::*;
use rstest::rstest;

#[rstest]
fn create_duplicate_branch(init_repository_dir: TempDir) -> Result<(), Box<dyn std::error::Error>> {
    let repository_dir = init_repository_dir;

    let branch_name = "feature-branch";

    // create the branch first time - should succeed
    run_bit_command(repository_dir.path(), &["branch", branch_name])
        .assert()
        .success();

    // assert the branch ref exists
    let branch_ref_path = repository_dir
        .path()
        .join(".git")
        .join("refs")
        .join("heads")
        .join(branch_name);
    assert!(branch_ref_path.exists());

    // attempt to create the same branch again - should fail
    run_bit_command(repository_dir.path(), &["branch", branch_name])
        .assert()
        .failure()
        .stderr(predicate::str::contains("already exists"));

    Ok(())
}
