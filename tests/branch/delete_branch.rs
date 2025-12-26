use crate::common::command::{init_repository_dir, run_bit_command};
use assert_fs::TempDir;
use predicates::prelude::*;
use rstest::rstest;

#[rstest]
fn delete_branch_successfully(
    init_repository_dir: TempDir,
) -> Result<(), Box<dyn std::error::Error>> {
    let repository_dir = init_repository_dir;

    // Create a branch
    run_bit_command(repository_dir.path(), &["branch", "create", "feature"])
        .assert()
        .success();

    // Verify the branch exists
    let branch_path = repository_dir
        .path()
        .join(".git")
        .join("refs")
        .join("heads")
        .join("feature");
    assert!(branch_path.exists());

    // Read the branch OID before deletion
    let branch_oid = std::fs::read_to_string(&branch_path)?;
    let branch_oid = branch_oid.trim();

    // Delete the branch
    let output = run_bit_command(
        repository_dir.path(),
        &["branch", "delete", "feature", "--force"],
    )
    .assert()
    .success();

    // Verify the output contains the deleted branch info
    let stdout = String::from_utf8(output.get_output().stdout.clone())?;
    assert!(
        stdout.contains("feature") && stdout.contains(&branch_oid[..7]),
        "Expected output to contain branch name 'feature' and OID prefix '{}', got: {}",
        &branch_oid[..7],
        stdout
    );

    // Verify the branch no longer exists
    assert!(!branch_path.exists());

    Ok(())
}

#[rstest]
fn delete_nonexistent_branch_fails(
    init_repository_dir: TempDir,
) -> Result<(), Box<dyn std::error::Error>> {
    let repository_dir = init_repository_dir;

    // Try to delete a branch that doesn't exist
    run_bit_command(
        repository_dir.path(),
        &["branch", "delete", "nonexistent", "--force"],
    )
    .assert()
    .failure()
    .stderr(predicate::str::contains("does not exist").or(predicate::str::contains("not found")));

    Ok(())
}

#[rstest]
fn delete_multiple_branches_sequentially(
    init_repository_dir: TempDir,
) -> Result<(), Box<dyn std::error::Error>> {
    let repository_dir = init_repository_dir;

    // Create multiple branches
    run_bit_command(repository_dir.path(), &["branch", "create", "feature1"])
        .assert()
        .success();
    run_bit_command(repository_dir.path(), &["branch", "create", "feature2"])
        .assert()
        .success();

    let branch1_path = repository_dir
        .path()
        .join(".git")
        .join("refs")
        .join("heads")
        .join("feature1");
    let branch2_path = repository_dir
        .path()
        .join(".git")
        .join("refs")
        .join("heads")
        .join("feature2");

    // Read OIDs before deletion
    let branch1_oid = std::fs::read_to_string(&branch1_path)?.trim().to_string();
    let branch2_oid = std::fs::read_to_string(&branch2_path)?.trim().to_string();

    // Delete first branch
    let output1 = run_bit_command(
        repository_dir.path(),
        &["branch", "delete", "feature1", "--force"],
    )
    .assert()
    .success();

    let stdout1 = String::from_utf8(output1.get_output().stdout.clone())?;
    assert!(
        stdout1.contains("feature1") && stdout1.contains(&branch1_oid[..7]),
        "Expected output to contain 'feature1' and OID prefix, got: {}",
        stdout1
    );

    assert!(!branch1_path.exists());
    assert!(branch2_path.exists());

    // Delete second branch
    let output2 = run_bit_command(
        repository_dir.path(),
        &["branch", "delete", "feature2", "--force"],
    )
    .assert()
    .success();

    let stdout2 = String::from_utf8(output2.get_output().stdout.clone())?;
    assert!(
        stdout2.contains("feature2") && stdout2.contains(&branch2_oid[..7]),
        "Expected output to contain 'feature2' and OID prefix, got: {}",
        stdout2
    );

    assert!(!branch2_path.exists());

    Ok(())
}

#[rstest]
fn delete_hierarchical_branch(
    init_repository_dir: TempDir,
) -> Result<(), Box<dyn std::error::Error>> {
    let repository_dir = init_repository_dir;

    // Create a hierarchical branch
    run_bit_command(
        repository_dir.path(),
        &["branch", "create", "feature/login"],
    )
    .assert()
    .success();

    // Verify the branch exists
    let branch_path = repository_dir
        .path()
        .join(".git")
        .join("refs")
        .join("heads")
        .join("feature")
        .join("login");
    assert!(branch_path.exists());

    // Read the branch OID before deletion
    let branch_oid = std::fs::read_to_string(&branch_path)?.trim().to_string();

    // Delete the hierarchical branch
    let output = run_bit_command(
        repository_dir.path(),
        &["branch", "delete", "feature/login", "--force"],
    )
    .assert()
    .success();

    // Verify the output contains the deleted branch info
    let stdout = String::from_utf8(output.get_output().stdout.clone())?;
    assert!(
        stdout.contains("feature/login") && stdout.contains(&branch_oid[..7]),
        "Expected output to contain 'feature/login' and OID prefix, got: {}",
        stdout
    );

    // Verify the branch no longer exists
    assert!(!branch_path.exists());

    Ok(())
}

#[rstest]
fn delete_branch_with_force_flag(
    init_repository_dir: TempDir,
) -> Result<(), Box<dyn std::error::Error>> {
    let repository_dir = init_repository_dir;

    // Create a branch
    run_bit_command(repository_dir.path(), &["branch", "create", "feature"])
        .assert()
        .success();

    let branch_path = repository_dir
        .path()
        .join(".git")
        .join("refs")
        .join("heads")
        .join("feature");

    // Read the branch OID before deletion
    let branch_oid = std::fs::read_to_string(&branch_path)?.trim().to_string();

    // Delete the branch with force flag
    let output = run_bit_command(
        repository_dir.path(),
        &["branch", "delete", "feature", "--force"],
    )
    .assert()
    .success();

    // Verify the output contains the deleted branch info
    let stdout = String::from_utf8(output.get_output().stdout.clone())?;
    assert!(
        stdout.contains("feature") && stdout.contains(&branch_oid[..7]),
        "Expected output to contain 'feature' and OID prefix, got: {}",
        stdout
    );

    assert!(!branch_path.exists());

    Ok(())
}

#[rstest]
fn delete_branch_with_force_short_flag(
    init_repository_dir: TempDir,
) -> Result<(), Box<dyn std::error::Error>> {
    let repository_dir = init_repository_dir;

    // Create a branch
    run_bit_command(repository_dir.path(), &["branch", "create", "feature"])
        .assert()
        .success();

    let branch_path = repository_dir
        .path()
        .join(".git")
        .join("refs")
        .join("heads")
        .join("feature");

    // Read the branch OID before deletion
    let branch_oid = std::fs::read_to_string(&branch_path)?.trim().to_string();

    // Delete the branch with force short flag
    let output = run_bit_command(
        repository_dir.path(),
        &["branch", "delete", "feature", "-f"],
    )
    .assert()
    .success();

    // Verify the output contains the deleted branch info
    let stdout = String::from_utf8(output.get_output().stdout.clone())?;
    assert!(
        stdout.contains("feature") && stdout.contains(&branch_oid[..7]),
        "Expected output to contain 'feature' and OID prefix, got: {}",
        stdout
    );

    assert!(!branch_path.exists());

    Ok(())
}

#[rstest]
fn delete_current_branch_fails(
    init_repository_dir: TempDir,
) -> Result<(), Box<dyn std::error::Error>> {
    let repository_dir = init_repository_dir;

    // Create a branch and check it out
    run_bit_command(repository_dir.path(), &["branch", "create", "main"])
        .assert()
        .success();

    run_bit_command(repository_dir.path(), &["checkout", "main"])
        .assert()
        .success();

    // Try to delete the current branch (should fail)
    run_bit_command(
        repository_dir.path(),
        &["branch", "delete", "main", "--force"],
    )
    .assert()
    .failure()
    .stderr(predicate::str::contains("current branch").or(predicate::str::contains("checked out")));

    // Verify the branch still exists
    let branch_path = repository_dir
        .path()
        .join(".git")
        .join("refs")
        .join("heads")
        .join("main");
    assert!(branch_path.exists());

    Ok(())
}

#[rstest]
fn delete_current_branch_with_force_still_fails(
    init_repository_dir: TempDir,
) -> Result<(), Box<dyn std::error::Error>> {
    let repository_dir = init_repository_dir;

    // Create a branch and check it out
    run_bit_command(repository_dir.path(), &["branch", "create", "main"])
        .assert()
        .success();

    run_bit_command(repository_dir.path(), &["checkout", "main"])
        .assert()
        .success();

    // Try to force delete the current branch (should still fail)
    run_bit_command(
        repository_dir.path(),
        &["branch", "delete", "main", "--force"],
    )
    .assert()
    .failure()
    .stderr(predicate::str::contains("current branch").or(predicate::str::contains("checked out")));

    // Verify the branch still exists
    let branch_path = repository_dir
        .path()
        .join(".git")
        .join("refs")
        .join("heads")
        .join("main");
    assert!(branch_path.exists());

    Ok(())
}

#[rstest]
fn delete_branch_that_is_not_current(
    init_repository_dir: TempDir,
) -> Result<(), Box<dyn std::error::Error>> {
    let repository_dir = init_repository_dir;

    // Create two branches
    run_bit_command(repository_dir.path(), &["branch", "create", "main"])
        .assert()
        .success();
    run_bit_command(repository_dir.path(), &["branch", "create", "feature"])
        .assert()
        .success();

    // Checkout main
    run_bit_command(repository_dir.path(), &["checkout", "main"])
        .assert()
        .success();

    let feature_path = repository_dir
        .path()
        .join(".git")
        .join("refs")
        .join("heads")
        .join("feature");

    // Read the feature branch OID before deletion
    let feature_oid = std::fs::read_to_string(&feature_path)?.trim().to_string();

    // Delete feature (not current branch) - should succeed
    let output = run_bit_command(
        repository_dir.path(),
        &["branch", "delete", "feature", "--force"],
    )
    .assert()
    .success();

    // Verify the output contains the deleted branch info
    let stdout = String::from_utf8(output.get_output().stdout.clone())?;
    assert!(
        stdout.contains("feature") && stdout.contains(&feature_oid[..7]),
        "Expected output to contain 'feature' and OID prefix, got: {}",
        stdout
    );

    assert!(!feature_path.exists());

    // Main should still exist
    let main_path = repository_dir
        .path()
        .join(".git")
        .join("refs")
        .join("heads")
        .join("main");
    assert!(main_path.exists());

    Ok(())
}

#[rstest]
fn delete_multiple_branches_at_once(
    init_repository_dir: TempDir,
) -> Result<(), Box<dyn std::error::Error>> {
    let repository_dir = init_repository_dir;

    // Create multiple branches
    run_bit_command(repository_dir.path(), &["branch", "create", "feature1"])
        .assert()
        .success();
    run_bit_command(repository_dir.path(), &["branch", "create", "feature2"])
        .assert()
        .success();
    run_bit_command(repository_dir.path(), &["branch", "create", "feature3"])
        .assert()
        .success();

    // Get paths for all branches
    let branch1_path = repository_dir
        .path()
        .join(".git")
        .join("refs")
        .join("heads")
        .join("feature1");
    let branch2_path = repository_dir
        .path()
        .join(".git")
        .join("refs")
        .join("heads")
        .join("feature2");
    let branch3_path = repository_dir
        .path()
        .join(".git")
        .join("refs")
        .join("heads")
        .join("feature3");

    // Read OIDs before deletion
    let branch1_oid = std::fs::read_to_string(&branch1_path)?.trim().to_string();
    let branch2_oid = std::fs::read_to_string(&branch2_path)?.trim().to_string();
    let branch3_oid = std::fs::read_to_string(&branch3_path)?.trim().to_string();

    // Delete multiple branches in one command
    let output = run_bit_command(
        repository_dir.path(),
        &[
            "branch", "delete", "feature1", "feature2", "feature3", "--force",
        ],
    )
    .assert()
    .success();

    // Verify the output contains all deleted branches info
    let stdout = String::from_utf8(output.get_output().stdout.clone())?;
    assert!(
        stdout.contains("feature1") && stdout.contains(&branch1_oid[..7]),
        "Expected output to contain 'feature1' and its OID prefix, got: {}",
        stdout
    );
    assert!(
        stdout.contains("feature2") && stdout.contains(&branch2_oid[..7]),
        "Expected output to contain 'feature2' and its OID prefix, got: {}",
        stdout
    );
    assert!(
        stdout.contains("feature3") && stdout.contains(&branch3_oid[..7]),
        "Expected output to contain 'feature3' and its OID prefix, got: {}",
        stdout
    );

    // Verify all branches are deleted
    assert!(!branch1_path.exists());
    assert!(!branch2_path.exists());
    assert!(!branch3_path.exists());

    Ok(())
}

#[rstest]
fn delete_multiple_branches_with_force(
    init_repository_dir: TempDir,
) -> Result<(), Box<dyn std::error::Error>> {
    let repository_dir = init_repository_dir;

    // Create multiple branches
    run_bit_command(repository_dir.path(), &["branch", "create", "feature1"])
        .assert()
        .success();
    run_bit_command(repository_dir.path(), &["branch", "create", "feature2"])
        .assert()
        .success();

    let branch1_path = repository_dir
        .path()
        .join(".git")
        .join("refs")
        .join("heads")
        .join("feature1");
    let branch2_path = repository_dir
        .path()
        .join(".git")
        .join("refs")
        .join("heads")
        .join("feature2");

    // Read OIDs before deletion
    let branch1_oid = std::fs::read_to_string(&branch1_path)?.trim().to_string();
    let branch2_oid = std::fs::read_to_string(&branch2_path)?.trim().to_string();

    // Delete multiple branches with force flag
    let output = run_bit_command(
        repository_dir.path(),
        &["branch", "delete", "feature1", "feature2", "--force"],
    )
    .assert()
    .success();

    // Verify the output contains all deleted branches info
    let stdout = String::from_utf8(output.get_output().stdout.clone())?;
    assert!(
        stdout.contains("feature1") && stdout.contains(&branch1_oid[..7]),
        "Expected output to contain 'feature1' and its OID prefix, got: {}",
        stdout
    );
    assert!(
        stdout.contains("feature2") && stdout.contains(&branch2_oid[..7]),
        "Expected output to contain 'feature2' and its OID prefix, got: {}",
        stdout
    );

    assert!(!branch1_path.exists());
    assert!(!branch2_path.exists());

    Ok(())
}

#[rstest]
fn delete_multiple_branches_including_current_fails(
    init_repository_dir: TempDir,
) -> Result<(), Box<dyn std::error::Error>> {
    let repository_dir = init_repository_dir;

    // Create branches
    run_bit_command(repository_dir.path(), &["branch", "create", "main"])
        .assert()
        .success();
    run_bit_command(repository_dir.path(), &["branch", "create", "feature"])
        .assert()
        .success();

    // Checkout main
    run_bit_command(repository_dir.path(), &["checkout", "main"])
        .assert()
        .success();

    // Try to delete multiple branches including current
    run_bit_command(
        repository_dir.path(),
        &["branch", "delete", "feature", "main", "--force"],
    )
    .assert()
    .failure()
    .stderr(predicate::str::contains("current branch").or(predicate::str::contains("checked out")));

    let main_path = repository_dir
        .path()
        .join(".git")
        .join("refs")
        .join("heads")
        .join("main");
    let feature_path = repository_dir
        .path()
        .join(".git")
        .join("refs")
        .join("heads")
        .join("feature");

    assert!(main_path.exists());
    assert!(!feature_path.exists());

    Ok(())
}

#[rstest]
fn delete_branch_with_invalid_name_fails(
    init_repository_dir: TempDir,
) -> Result<(), Box<dyn std::error::Error>> {
    let repository_dir = init_repository_dir;

    // Try to delete a branch with invalid name characters
    run_bit_command(
        repository_dir.path(),
        &["branch", "delete", "feature..bad", "--force"],
    )
    .assert()
    .failure()
    .stderr(predicate::str::contains("invalid").or(predicate::str::contains("does not exist")));

    Ok(())
}
