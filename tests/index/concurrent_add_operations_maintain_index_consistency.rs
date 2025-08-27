use assert_cmd::Command;
use assert_fs::fixture::{FileWriteStr, PathChild};
use predicates::prelude::predicate;
use tokio::time::Duration;
use crate::{assert_index_eq, common};

#[tokio::test]
async fn concurrent_add_operations_maintain_index_consistency()
    -> Result<(), Box<dyn std::error::Error>> {
    common::redirect_temp_dir();
    let dir = assert_fs::TempDir::new()?;
    let mut cmd = Command::cargo_bin("bit")?;
    cmd.current_dir(dir.path()).arg("init");

    cmd.assert().success().stdout(predicate::str::contains(
        "Initialized empty Git repository in",
    ));

    // Create alice.rb and bob.py files
    let alice_file = dir.child("alice.rb");
    alice_file.write_str("puts 'Hello from Alice'")?;

    let bob_file = dir.child("bob.py");
    bob_file.write_str("print('Hello from Bob')")?;

    // Simulate concurrent add operations using Tokio tasks
    // This tests the index locking behavior where:
    // 1. Alice starts add operation (reads current index)
    // 2. Bob starts add operation (also reads same index state)
    // 3. Alice completes first (acquires lock, writes index)
    // 4. Bob completes second (should see Alice's changes and merge properly)

    let dir_path = dir.path().to_path_buf();
    let dir_path_clone = dir_path.clone();

    // Launch both operations concurrently
    let (alice_result, bob_result) = tokio::join!(
        // Alice's add operation
        tokio::spawn(async move {
            let mut alice_cmd = Command::cargo_bin("bit").unwrap();
            alice_cmd
                .current_dir(&dir_path)
                .arg("add")
                .arg("alice.rb")
                .assert()
                .success();
        }),
        // Bob's add operation with small delay to ensure overlap
        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(10)).await;
            let mut bob_cmd = Command::cargo_bin("bit").unwrap();
            bob_cmd
                .current_dir(&dir_path_clone)
                .arg("add")
                .arg("bob.py")
                .assert()
                .success();
        })
    );

    // Ensure both tasks completed successfully
    alice_result.expect("Alice's task should complete successfully");
    bob_result.expect("Bob's task should complete successfully");

    // Read the final index state
    let bit_index_path = dir.child(".git/index");
    let bit_index_content = std::fs::read(bit_index_path.path())?;

    // Compare with what git would produce for the same operations
    std::fs::remove_dir_all(dir.child(".git").path())?;
    let mut git_cmd = Command::new("git");
    git_cmd.current_dir(dir.path()).arg("init");
    git_cmd.assert().success();

    // Add both files with git in the same order
    let mut git_add_cmd = Command::new("git");
    git_add_cmd
        .current_dir(dir.path())
        .arg("add")
        .arg("alice.rb")
        .assert()
        .success();

    let mut git_add_cmd2 = Command::new("git");
    git_add_cmd2
        .current_dir(dir.path())
        .arg("add")
        .arg("bob.py")
        .assert()
        .success();

    // Compare final index contents
    let git_index_path = dir.child(".git/index");
    let git_index_content = std::fs::read(git_index_path.path())?;
    assert_index_eq!(
        &bit_index_content,
        &git_index_content,
        "Concurrent add operations should result in consistent index with both files"
    );

    Ok(())
}
