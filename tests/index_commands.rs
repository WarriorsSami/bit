use assert_cmd::Command;
use assert_fs::fixture::{FileWriteStr, PathChild};
use assert_fs::prelude::PathCreateDir;
use fake::Fake;
use fake::faker::lorem::en::{Word, Words};
use predicates::prelude::predicate;
use tokio::time::{Duration, sleep};

mod common;

#[test]
fn add_single_file_to_index_successfully() -> Result<(), Box<dyn std::error::Error>> {
    common::redirect_temp_dir();
    let dir = assert_fs::TempDir::new()?;
    let mut cmd = Command::cargo_bin("bit")?;
    cmd.current_dir(dir.path()).arg("init");

    cmd.assert().success().stdout(predicate::str::contains(
        "Initialized empty Git repository in",
    ));

    let file_name = format!("{}.txt", Word().fake::<String>());
    let file_path = dir.child(file_name.clone());
    let file_content = Words(5..10).fake::<Vec<String>>().join(" ");
    file_path.write_str(&file_content.clone())?;

    // Add the file to the index using bit
    let mut sut = Command::cargo_bin("bit")?;

    sut.current_dir(dir.path())
        .arg("add")
        .arg(&file_name)
        .assert()
        .success();

    // Store the index content
    let bit_index_path = dir.child(".git/index");
    let bit_index_content = std::fs::read(bit_index_path.path())?;

    // Delete the .git directory and recreate it using git
    std::fs::remove_dir_all(dir.child(".git").path())?;
    let mut git_cmd = Command::new("git");
    git_cmd.current_dir(dir.path()).arg("init");
    git_cmd.assert().success();

    // Add the file to the index using git
    let mut git_add_cmd = Command::new("git");
    git_add_cmd
        .current_dir(dir.path())
        .arg("add")
        .arg(&file_name)
        .assert()
        .success();

    // Compare the index content with the git index content
    let git_index_path = dir.child(".git/index");
    let git_index_content = std::fs::read(git_index_path.path())?;
    assert_index_eq!(&bit_index_content, &git_index_content);

    Ok(())
}

#[test]
fn add_multiple_files_to_index_successfully() -> Result<(), Box<dyn std::error::Error>> {
    common::redirect_temp_dir();
    let dir = assert_fs::TempDir::new()?;
    let mut cmd = Command::cargo_bin("bit")?;
    cmd.current_dir(dir.path()).arg("init");

    cmd.assert().success().stdout(predicate::str::contains(
        "Initialized empty Git repository in",
    ));

    let file_names: Vec<String> = (0..5)
        .map(|_| format!("{}.txt", Word().fake::<String>()))
        .collect();

    for file_name in &file_names {
        let file_path = dir.child(file_name.clone());
        let file_content = Words(5..10).fake::<Vec<String>>().join(" ");
        file_path.write_str(&file_content)?;
    }

    // Add the files to the index using bit
    let mut sut = Command::cargo_bin("bit")?;

    sut.current_dir(dir.path())
        .arg("add")
        .args(&file_names)
        .assert()
        .success();

    // Store the index content
    let bit_index_path = dir.child(".git/index");
    let bit_index_content = std::fs::read(bit_index_path.path())?;

    // Delete the .git directory and recreate it using git
    std::fs::remove_dir_all(dir.child(".git").path())?;
    let mut git_cmd = Command::new("git");
    git_cmd.current_dir(dir.path()).arg("init");
    git_cmd.assert().success();

    // Add the files to the index using git
    let mut git_add_cmd = Command::new("git");
    git_add_cmd
        .current_dir(dir.path())
        .arg("add")
        .args(&file_names)
        .assert()
        .success();

    // Compare the index content with the git index content
    let git_index_path = dir.child(".git/index");
    let git_index_content = std::fs::read(git_index_path.path())?;
    assert_index_eq!(&bit_index_content, &git_index_content);

    Ok(())
}

#[test]
fn add_files_in_nested_directories_to_index_successfully() -> Result<(), Box<dyn std::error::Error>>
{
    common::redirect_temp_dir();
    let dir = assert_fs::TempDir::new()?;
    let mut cmd = Command::cargo_bin("bit")?;
    cmd.current_dir(dir.path()).arg("init");

    cmd.assert().success().stdout(predicate::str::contains(
        "Initialized empty Git repository in",
    ));

    // Create nested directories and files
    let mut file_names = Vec::new();
    let dir_count = (1..=5).fake::<usize>();
    for _ in 0..dir_count {
        let dir_name = Word().fake::<String>();
        let dir_path = dir.child(dir_name.clone());
        dir_path.create_dir_all()?;
        let file_count = (1..=5).fake::<usize>();
        for _ in 0..file_count {
            let file_name = format!("{}.txt", Word().fake::<String>());
            let file_path = dir_path.child(file_name.clone());
            let file_content = Words(5..10).fake::<Vec<String>>().join(" ");
            file_path.write_str(&file_content.clone())?;
            file_names.push(format!("{dir_name}/{file_name}"));
        }
    }

    // Add the files to the index using bit
    let mut sut = Command::cargo_bin("bit")?;

    sut.current_dir(dir.path())
        .arg("add")
        .arg(".")
        .assert()
        .success();

    // Store the index content
    let bit_index_path = dir.child(".git/index");
    let bit_index_content = std::fs::read(bit_index_path.path())?;

    // Delete the .git directory and recreate it using git
    std::fs::remove_dir_all(dir.child(".git").path())?;
    let mut git_cmd = Command::new("git");
    git_cmd.current_dir(dir.path()).arg("init");
    git_cmd.assert().success();

    // Add the files to the index using git
    let mut git_add_cmd = Command::new("git");
    git_add_cmd
        .current_dir(dir.path())
        .arg("add")
        .arg(".")
        .assert()
        .success();

    // Compare the index content with the git index content
    let git_index_path = dir.child(".git/index");
    let git_index_content = std::fs::read(git_index_path.path())?;
    assert_index_eq!(&bit_index_content, &git_index_content);

    Ok(())
}

#[test]
fn add_multiple_files_to_index_incrementally_successfully() -> Result<(), Box<dyn std::error::Error>>
{
    common::redirect_temp_dir();
    let dir = assert_fs::TempDir::new()?;
    let mut cmd = Command::cargo_bin("bit")?;
    cmd.current_dir(dir.path()).arg("init");

    cmd.assert().success().stdout(predicate::str::contains(
        "Initialized empty Git repository in",
    ));

    let file_names: Vec<String> = (0..5)
        .map(|_| format!("{}.txt", Word().fake::<String>()))
        .collect();

    for file_name in &file_names {
        let file_path = dir.child(file_name.clone());
        let file_content = Words(5..10).fake::<Vec<String>>().join(" ");
        file_path.write_str(&file_content)?;
    }

    // Add the first two files to the index using bit
    let mut sut = Command::cargo_bin("bit")?;

    sut.current_dir(dir.path())
        .arg("add")
        .args(&file_names[..2])
        .assert()
        .success();

    // Add the remaining files incrementally
    let mut sut = Command::cargo_bin("bit")?;

    sut.current_dir(dir.path())
        .arg("add")
        .args(&file_names[2..])
        .assert()
        .success();

    // Store the index content
    let bit_index_path = dir.child(".git/index");
    let bit_index_content = std::fs::read(bit_index_path.path())?;

    // Delete the .git directory and recreate it using git
    std::fs::remove_dir_all(dir.child(".git").path())?;
    let mut git_cmd = Command::new("git");
    git_cmd.current_dir(dir.path()).arg("init");
    git_cmd.assert().success();

    // Add the first two files to the index using git
    let mut git_add_cmd = Command::new("git");

    git_add_cmd
        .current_dir(dir.path())
        .arg("add")
        .args(&file_names[..2])
        .assert()
        .success();

    // Add the remaining files incrementally
    let mut git_add_cmd = Command::new("git");

    git_add_cmd
        .current_dir(dir.path())
        .arg("add")
        .args(&file_names[2..])
        .assert()
        .success();

    // Compare the index content with the git index content
    let git_index_path = dir.child(".git/index");
    let git_index_content = std::fs::read(git_index_path.path())?;
    assert_index_eq!(&bit_index_content, &git_index_content);

    Ok(())
}

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
            sleep(Duration::from_millis(10)).await;
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

#[test]
fn replace_file_with_directory_successfully() -> Result<(), Box<dyn std::error::Error>> {
    common::redirect_temp_dir();
    let dir = assert_fs::TempDir::new()?;
    let mut cmd = Command::cargo_bin("bit")?;
    cmd.current_dir(dir.path()).arg("init");

    cmd.assert().success().stdout(predicate::str::contains(
        "Initialized empty Git repository in",
    ));

    // Create a file and add it to the index
    let file_name = format!("{}.txt", Word().fake::<String>());
    let file_path = dir.child(file_name.clone());
    let file_content = Words(5..10).fake::<Vec<String>>().join(" ");
    file_path.write_str(&file_content)?;

    // Add the file to the index using bit
    let mut sut = Command::cargo_bin("bit")?;
    sut.current_dir(dir.path())
        .arg("add")
        .arg(&file_name)
        .assert()
        .success();

    // Create a directory with the same name as the file
    let dir_name = file_name;
    let dir_path = dir.child(dir_name.clone());

    // Remove the file and create a directory with the same name
    std::fs::remove_file(file_path.path())?;

    // Create the directory and add a file inside it
    dir_path.create_dir_all()?;
    let nested_file_name = format!("nested_{}.txt", Word().fake::<String>());
    let nested_file_path = dir_path.child(nested_file_name.clone());
    let nested_file_content = Words(5..10).fake::<Vec<String>>().join(" ");
    nested_file_path.write_str(&nested_file_content)?;

    // Attempt to add the directory to the index using bit
    let mut sut = Command::cargo_bin("bit")?;
    sut.current_dir(dir.path())
        .arg("add")
        .arg(&dir_name)
        .assert()
        .success();

    // Store the index content
    let bit_index_path = dir.child(".git/index");
    let bit_index_content = std::fs::read(bit_index_path.path())?;

    // Delete the .git directory and recreate it using git
    std::fs::remove_dir_all(dir.child(".git").path())?;
    let mut git_cmd = Command::new("git");
    git_cmd.current_dir(dir.path()).arg("init");
    git_cmd.assert().success();

    // Add the directory to the index using git
    let mut git_add_cmd = Command::new("git");
    git_add_cmd
        .current_dir(dir.path())
        .arg("add")
        .arg(&dir_name)
        .assert()
        .success();

    // Compare the index content with the git index content
    let git_index_path = dir.child(".git/index");
    let git_index_content = std::fs::read(git_index_path.path())?;
    assert_index_eq!(&bit_index_content, &git_index_content);

    Ok(())
}

#[test]
fn replace_directory_having_only_direct_children_with_file_successfully()
-> Result<(), Box<dyn std::error::Error>> {
    common::redirect_temp_dir();
    let dir = assert_fs::TempDir::new()?;
    let mut cmd = Command::cargo_bin("bit")?;
    cmd.current_dir(dir.path()).arg("init");

    cmd.assert().success().stdout(predicate::str::contains(
        "Initialized empty Git repository in",
    ));

    // Create a file and add it to the index
    let file_name = format!("{}.txt", Word().fake::<String>());
    let file_path = dir.child(file_name.clone());
    let file_content = Words(5..10).fake::<Vec<String>>().join(" ");
    file_path.write_str(&file_content)?;

    // Add the file to the index using bit
    let mut sut = Command::cargo_bin("bit")?;
    sut.current_dir(dir.path())
        .arg("add")
        .arg(&file_name)
        .assert()
        .success();

    // Create a directory and add a file inside it
    let dir_name = format!("{}.dir", Word().fake::<String>());
    let dir_path = dir.child(dir_name.clone());
    dir_path.create_dir_all()?;

    let nested_file_name = format!("nested_{}.txt", Word().fake::<String>());
    let nested_file_path = dir_path.child(nested_file_name.clone());
    let nested_file_content = Words(5..10).fake::<Vec<String>>().join(" ");
    nested_file_path.write_str(&nested_file_content)?;

    // Add the file to the index using bit
    let mut sut = Command::cargo_bin("bit")?;
    sut.current_dir(dir.path())
        .arg("add")
        .arg(nested_file_path.path())
        .assert()
        .success();

    // Remove the directory and create a file with the same name
    std::fs::remove_dir_all(dir_path.path())?;
    let new_file_path = dir.child(dir_name.clone());
    new_file_path.write_str(&nested_file_content)?;

    // Attempt to add the new file to the index using bit
    let mut sut = Command::cargo_bin("bit")?;
    sut.current_dir(dir.path())
        .arg("add")
        .arg(new_file_path.path())
        .assert()
        .success();

    // Store the index content
    let bit_index_path = dir.child(".git/index");
    let bit_index_content = std::fs::read(bit_index_path.path())?;

    // Delete the .git directory and recreate it using git
    std::fs::remove_dir_all(dir.child(".git").path())?;

    let mut git_cmd = Command::new("git");
    git_cmd
        .current_dir(dir.path())
        .arg("init")
        .assert()
        .success();

    // Add the new file to the index using git
    let mut git_add_cmd = Command::new("git");
    git_add_cmd
        .current_dir(dir.path())
        .arg("add")
        .arg(".")
        .assert()
        .success();

    // Compare the index content with the git index content
    let git_index_path = dir.child(".git/index");
    let git_index_content = std::fs::read(git_index_path.path())?;
    assert_index_eq!(&bit_index_content, &git_index_content);

    Ok(())
}

#[test]
fn replace_directory_having_nested_children_with_file_successfully()
-> Result<(), Box<dyn std::error::Error>> {
    common::redirect_temp_dir();
    let dir = assert_fs::TempDir::new()?;
    let mut cmd = Command::cargo_bin("bit")?;
    cmd.current_dir(dir.path()).arg("init");

    cmd.assert().success().stdout(predicate::str::contains(
        "Initialized empty Git repository in",
    ));

    // Create a file and add it to the index
    let file_name = format!("{}.txt", Word().fake::<String>());
    let file_path = dir.child(file_name.clone());
    let file_content = Words(5..10).fake::<Vec<String>>().join(" ");
    file_path.write_str(&file_content)?;

    // Add the file to the index using bit
    let mut sut = Command::cargo_bin("bit")?;
    sut.current_dir(dir.path())
        .arg("add")
        .arg(&file_name)
        .assert()
        .success();

    // Create a directory and add a nested directory with a file inside it
    let dir_name = format!("{}.dir", Word().fake::<String>());
    let dir_path = dir.child(dir_name.clone());
    dir_path.create_dir_all()?;

    let nested_dir_name = format!("nested_{}.dir", Word().fake::<String>());
    let nested_dir_path = dir_path.child(nested_dir_name.clone());
    nested_dir_path.create_dir_all()?;

    let nested_file_name = format!("nested_file_{}.txt", Word().fake::<String>());
    let nested_file_path = nested_dir_path.child(nested_file_name.clone());
    let nested_file_content = Words(5..10).fake::<Vec<String>>().join(" ");
    nested_file_path.write_str(&nested_file_content)?;

    // Add the nested file to the index using bit
    let mut sut = Command::cargo_bin("bit")?;
    sut.current_dir(dir.path())
        .arg("add")
        .arg(nested_file_path.path())
        .assert()
        .success();

    // Remove the directory and create a file with the same name
    std::fs::remove_dir_all(dir_path.path())?;
    let new_file_path = dir.child(dir_name.clone());
    new_file_path.write_str(&nested_file_content)?;

    // Attempt to add the new file to the index using bit
    let mut sut = Command::cargo_bin("bit")?;
    sut.current_dir(dir.path())
        .arg("add")
        .arg(new_file_path.path())
        .assert()
        .success();

    // Store the index content
    let bit_index_path = dir.child(".git/index");
    let bit_index_content = std::fs::read(bit_index_path.path())?;

    // Delete the .git directory and recreate it using git
    std::fs::remove_dir_all(dir.child(".git").path())?;
    let mut git_cmd = Command::new("git");
    git_cmd
        .current_dir(dir.path())
        .arg("init")
        .assert()
        .success();

    // Add the new file to the index using git
    let mut git_add_cmd = Command::new("git");
    git_add_cmd
        .current_dir(dir.path())
        .arg("add")
        .arg(".")
        .assert()
        .success();

    // Compare the index content with the git index content
    let git_index_path = dir.child(".git/index");
    let git_index_content = std::fs::read(git_index_path.path())?;
    assert_index_eq!(&bit_index_content, &git_index_content);

    Ok(())
}
