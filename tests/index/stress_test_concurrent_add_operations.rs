use assert_cmd::Command;
use assert_fs::fixture::{FileWriteStr, PathChild};
use fake::Fake;
use predicates::prelude::predicate;
use tokio::time::{Duration, sleep};
use crate::{assert_index_eq, common};

#[tokio::test]
async fn stress_test_concurrent_add_operations() -> Result<(), Box<dyn std::error::Error>> {
    common::redirect_temp_dir();
    let dir = assert_fs::TempDir::new()?;
    let mut cmd = Command::cargo_bin("bit")?;
    cmd.current_dir(dir.path()).arg("init");

    cmd.assert().success().stdout(predicate::str::contains(
        "Initialized empty Git repository in",
    ));

    // Create multiple files to be added concurrently
    let file_count = 10;
    let file_names: Vec<String> = (0..file_count).map(|i| format!("file_{}.txt", i)).collect();

    for file_name in &file_names {
        let file_path = dir.child(file_name);
        let file_content = format!("Content of {}", file_name);
        file_path.write_str(&file_content)?;
    }

    // Launch multiple concurrent add operations using Tokio tasks
    let mut tasks = Vec::new();

    for file_name in &file_names {
        let dir_path = dir.path().to_path_buf();
        let file_name_clone = file_name.clone();

        let task = tokio::spawn(async move {
            // Add small random delay to increase chance of overlap
            let delay_ms = (0..50).fake::<u64>();
            sleep(Duration::from_millis(delay_ms)).await;

            let mut add_cmd = Command::cargo_bin("bit").unwrap();
            add_cmd
                .current_dir(&dir_path)
                .arg("add")
                .arg(&file_name_clone)
                .assert()
                .success();
        });

        tasks.push(task);
    }

    // Wait for ALL tasks to complete concurrently using join_all
    let results = futures::future::join_all(tasks).await;

    // Check that all tasks completed successfully
    for result in results {
        result.expect("Add operation should complete successfully");
    }

    // Verify final index state matches git
    let bit_index_path = dir.child(".git/index");
    let bit_index_content = std::fs::read(bit_index_path.path())?;

    // Create reference git index
    std::fs::remove_dir_all(dir.child(".git").path())?;
    let mut git_cmd = Command::new("git");
    git_cmd.current_dir(dir.path()).arg("init");
    git_cmd.assert().success();

    // Add all files to git index
    for file_name in &file_names {
        let mut git_add_cmd = Command::new("git");
        git_add_cmd
            .current_dir(dir.path())
            .arg("add")
            .arg(file_name)
            .assert()
            .success();
    }

    let git_index_path = dir.child(".git/index");
    let git_index_content = std::fs::read(git_index_path.path())?;
    assert_index_eq!(
        &bit_index_content,
        &git_index_content,
        "Stress test: All {} files should be present in index after concurrent operations",
        file_count
    );

    Ok(())
}

