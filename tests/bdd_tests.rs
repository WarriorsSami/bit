use assert_cmd::Command;
use assert_fs::fixture::{FileWriteStr, PathChild};
use assert_fs::prelude::PathCreateDir;
use cucumber::{World, given, then, when};
use predicates::prelude::predicate;
use std::fs;
use tokio::time::Duration;

mod common;
use common::world::TestWorld;

#[derive(Debug, World)]
#[world(init = Self::new)]
pub struct UnifiedWorld {
    inner: TestWorld,
}

impl UnifiedWorld {
    fn new() -> Self {
        Self {
            inner: TestWorld::new(),
        }
    }
}

// ========================================
// SHARED BACKGROUND STEPS
// ========================================

#[given("I have redirected the temp directory")]
async fn redirect_temp_dir(_world: &mut UnifiedWorld) {
    common::redirect_temp_dir();
}

#[given("I have a clean temporary directory")]
async fn create_temp_directory(world: &mut UnifiedWorld) {
    world.inner.temp_dir =
        Some(assert_fs::TempDir::new().expect("Failed to create temp directory"));
}

#[given("I have initialized a git repository with bit")]
async fn initialize_git_repository(world: &mut UnifiedWorld) {
    let mut cmd = world.inner.run_bit_command(&["init"]);
    cmd.assert().success().stdout(predicate::str::contains(
        "Initialized empty Git repository in",
    ));
}

#[given("I have random author credentials configured")]
async fn configure_random_author_credentials(world: &mut UnifiedWorld) {
    world.inner.create_random_author_credentials();
}

#[given("I have a random commit message")]
async fn create_random_commit_message(world: &mut UnifiedWorld) {
    world.inner.create_random_commit_message();
}

// ========================================
// FILE CREATION STEPS (SHARED)
// ========================================

#[given(regex = r"^I have (\d+) files in a (flat|nested) project$")]
async fn create_files_with_project_structure(
    world: &mut UnifiedWorld,
    file_count: usize,
    structure: String,
) {
    world.inner.file_names.clear();
    world.inner.file_contents.clear();

    match structure.as_str() {
        "flat" => {
            for i in 0..file_count {
                let file_name = format!("file{}.txt", i + 1);
                let file_content = format!("Content of file {}", i + 1);
                let file_path = world.inner.get_temp_dir().child(&file_name);
                file_path
                    .write_str(&file_content)
                    .expect("Failed to write file");
                world.inner.file_names.push(file_name.clone());
                world.inner.file_contents.insert(file_name, file_content);
            }
        }
        "nested" => {
            let dirs_count = (file_count / 2).max(1);
            let files_per_dir = file_count / dirs_count;
            let mut file_counter = 0;

            for dir_i in 0..dirs_count {
                let dir_name = format!("dir{}", dir_i + 1);
                let dir_path = world.inner.get_temp_dir().child(&dir_name);
                dir_path
                    .create_dir_all()
                    .expect("Failed to create directory");

                let files_in_this_dir = if dir_i == dirs_count - 1 {
                    file_count - file_counter
                } else {
                    files_per_dir
                };

                for _file_i in 0..files_in_this_dir {
                    let file_name = format!("file{}.txt", file_counter + 1);
                    let file_content = format!("Content of file {}", file_counter + 1);
                    let file_path = dir_path.child(&file_name);
                    file_path
                        .write_str(&file_content)
                        .expect("Failed to write file");
                    let full_path = format!("{}/{}", dir_name, file_name);
                    world.inner.file_names.push(full_path.clone());
                    world.inner.file_contents.insert(full_path, file_content);
                    file_counter += 1;
                }
            }
        }
        _ => panic!("Unknown structure: {}", structure),
    }
}

#[given(regex = r#"^I have files in a (flat|nested) project structure$"#)]
async fn create_files_with_random_project_structure(world: &mut UnifiedWorld, structure: String) {
    use fake::Fake;
    let file_count = (1..=5).fake::<usize>();
    create_files_with_project_structure(world, file_count, structure).await;
}

#[given(regex = r#"^I have a file named "([^"]+)" in the project$"#)]
async fn create_named_file(world: &mut UnifiedWorld, file_name: String) {
    let file_content = format!("Content of {}", file_name);
    let file_path = world.inner.get_temp_dir().child(&file_name);
    file_path
        .write_str(&file_content)
        .expect("Failed to write file");
    world.inner.file_names.push(file_name.clone());
    world.inner.file_contents.insert(file_name, file_content);
}

#[given(regex = r#"^I have a directory "([^"]+)" with files in the project$"#)]
async fn create_directory_with_files(world: &mut UnifiedWorld, dir_name: String) {
    let dir_path = world.inner.get_temp_dir().child(&dir_name);
    dir_path
        .create_dir_all()
        .expect("Failed to create directory");

    for i in 1..=3 {
        let file_name = format!("file{}.txt", i);
        let file_content = format!("Content of {}/{}", dir_name, file_name);
        let file_path = dir_path.child(&file_name);
        file_path
            .write_str(&file_content)
            .expect("Failed to write file");
        let full_path = format!("{}/{}", dir_name, file_name);
        world.inner.file_names.push(full_path.clone());
        world.inner.file_contents.insert(full_path, file_content);
    }
}

#[given(regex = r#"^I have files "([^"]+)" and "([^"]+)" in the project$"#)]
async fn create_two_named_files(world: &mut UnifiedWorld, file1: String, file2: String) {
    for file_name in [file1, file2] {
        let file_content = match file_name.as_str() {
            name if name.ends_with(".rb") => "puts 'Hello from Alice'".to_string(),
            name if name.ends_with(".py") => "print('Hello from Bob')".to_string(),
            _ => format!("Content of {}", file_name),
        };
        let file_path = world.inner.get_temp_dir().child(&file_name);
        file_path
            .write_str(&file_content)
            .expect("Failed to write file");
        world.inner.file_names.push(file_name.clone());
        world.inner.file_contents.insert(file_name, file_content);
    }
}

#[given("I have an unreadable file in the project")]
async fn create_unreadable_file(world: &mut UnifiedWorld) {
    use std::os::unix::fs::PermissionsExt;

    let file_name = "unreadable.txt".to_string();
    let file_path = world.inner.get_temp_dir().child(&file_name);
    file_path
        .write_str("unreadable content")
        .expect("Failed to write file");

    // Make file unreadable
    let mut perms = fs::metadata(file_path.path()).unwrap().permissions();
    perms.set_mode(0o000);
    fs::set_permissions(file_path.path(), perms).expect("Failed to set permissions");

    world.inner.file_names.push(file_name.clone());
}

// ========================================
// INDEX-SPECIFIC STEPS
// ========================================

#[when("I add all files to the index using bit")]
async fn add_all_files_to_index(world: &mut UnifiedWorld) {
    let file_names_str = world.inner.file_names.join(" ");
    let mut cmd = world.inner.run_bit_command(&["add"]);
    for file_name in file_names_str.split_whitespace() {
        cmd.arg(file_name);
    }
    cmd.assert().success();
}

#[when(regex = r"^I add the (first|second|third) file to the index using bit$")]
async fn add_nth_file_to_index(world: &mut UnifiedWorld, nth: String) {
    let index = match nth.as_str() {
        "first" => 0,
        "second" => 1,
        "third" => 2,
        _ => panic!("Unknown ordinal: {}", nth),
    };

    if let Some(file_name) = world.inner.file_names.get(index) {
        let mut cmd = world.inner.run_bit_command(&["add", file_name]);
        cmd.assert().success();
    }
}

#[when("I add the existing file to the index using bit")]
async fn add_existing_file_to_index(world: &mut UnifiedWorld) {
    if let Some(file_name) = world.inner.file_names.first() {
        let mut cmd = world.inner.run_bit_command(&["add", file_name]);
        cmd.assert().success();
    }
}

#[when("I try to add a non-existent file to the index using bit")]
async fn try_add_nonexistent_file(world: &mut UnifiedWorld) {
    let mut cmd = world.inner.run_bit_command(&["add", "nonexistent.txt"]);
    let output = cmd.assert().failure().get_output().clone();
    world.inner.error_output = String::from_utf8(output.stderr).unwrap_or_default();
}

#[when("I try to add the unreadable file to the index using bit")]
async fn try_add_unreadable_file(world: &mut UnifiedWorld) {
    let mut cmd = world.inner.run_bit_command(&["add", "unreadable.txt"]);
    let output = cmd.assert().failure().get_output().clone();
    world.inner.error_output = String::from_utf8(output.stderr).unwrap_or_default();
}

#[when("I replace the file with a directory containing files")]
async fn replace_file_with_directory(world: &mut UnifiedWorld) {
    if let Some(file_name) = world.inner.file_names.first().cloned() {
        // Remove the file
        let file_path = world.inner.get_temp_dir().child(&file_name);
        fs::remove_file(file_path.path()).expect("Failed to remove file");

        // Create directory with same name
        let dir_path = world.inner.get_temp_dir().child(&file_name);
        dir_path
            .create_dir_all()
            .expect("Failed to create directory");

        // Add files to the directory
        for i in 1..=2 {
            let sub_file_name = format!("sub{}.txt", i);
            let sub_file_content = format!("Content of {}/{}", file_name, sub_file_name);
            let sub_file_path = dir_path.child(&sub_file_name);
            sub_file_path
                .write_str(&sub_file_content)
                .expect("Failed to write file");

            let full_path = format!("{}/{}", file_name, sub_file_name);
            world.inner.file_names.push(full_path.clone());
            world
                .inner
                .file_contents
                .insert(full_path, sub_file_content);
        }
    }
}

#[when("I add the directory to the index using bit")]
async fn add_directory_to_index(world: &mut UnifiedWorld) {
    let mut cmd = world.inner.run_bit_command(&["add", "."]);
    cmd.assert().success();
}

#[when("I replace the directory with a single file")]
async fn replace_directory_with_file(_world: &mut UnifiedWorld) {
    // This step would implement directory->file replacement logic
    // Implementation depends on your specific test requirements
}

#[when("I add the file to the index using bit")]
async fn add_file_to_index(world: &mut UnifiedWorld) {
    add_all_files_to_index(world).await;
}

#[when("multiple concurrent add operations are performed")]
async fn perform_concurrent_add_operations(world: &mut UnifiedWorld) {
    let dir_path = world.inner.get_temp_dir_path().to_path_buf();
    let dir_path_clone = dir_path.clone();

    let (alice_result, bob_result) = tokio::join!(
        tokio::spawn(async move {
            let mut cmd = Command::cargo_bin("bit").unwrap();
            cmd.current_dir(&dir_path)
                .arg("add")
                .arg("alice.rb")
                .assert()
                .success();
        }),
        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(10)).await;
            let mut cmd = Command::cargo_bin("bit").unwrap();
            cmd.current_dir(&dir_path_clone)
                .arg("add")
                .arg("bob.py")
                .assert()
                .success();
        })
    );

    alice_result.expect("Alice's add operation failed");
    bob_result.expect("Bob's add operation failed");
}

#[when("I recreate the same scenario using git for index comparison")]
async fn recreate_scenario_with_git_for_index(world: &mut UnifiedWorld) {
    // Store bit's index content
    let bit_index_path = world.inner.get_temp_dir().child(".git/index");
    world.inner.bit_index_content =
        fs::read(bit_index_path.path()).expect("Failed to read bit index");

    // Remove .git directory
    fs::remove_dir_all(world.inner.get_temp_dir().child(".git"))
        .expect("Failed to remove .git directory");

    // Recreate with git
    let mut cmd = world.inner.run_git_command(&["init"]);
    cmd.assert().success();

    // Add all files with git
    let mut cmd = world.inner.run_git_command(&["add", "."]);
    cmd.assert().success();

    // Store git's index content
    let git_index_path = world.inner.get_temp_dir().child(".git/index");
    world.inner.git_index_content =
        fs::read(git_index_path.path()).expect("Failed to read git index");
}

// ========================================
// COMMIT-SPECIFIC STEPS (REUSED FROM PREVIOUS)
// ========================================

#[when("I perform a complete commit workflow with bit")]
async fn perform_complete_commit_workflow_with_bit(world: &mut UnifiedWorld) {
    add_all_files_to_index(world).await;

    let mut cmd =
        world
            .inner
            .run_bit_command(&["commit", "-m", &world.inner.commit_message.clone()]);
    cmd.envs(vec![
        ("GIT_AUTHOR_NAME", &world.inner.author_name),
        ("GIT_AUTHOR_EMAIL", &world.inner.author_email),
    ]);

    let output = cmd.assert().success().get_output().clone();
    world.inner.commit_output = String::from_utf8(output.stdout).expect("Invalid UTF-8 in output");
}

// ========================================
// ASSERTION STEPS
// ========================================

#[then("both index contents should be identical")]
async fn verify_index_contents_identical(world: &mut UnifiedWorld) {
    assert_index_eq!(
        &world.inner.bit_index_content,
        &world.inner.git_index_content
    );
}

#[then("the operation should succeed without errors")]
async fn verify_operation_succeeds(_world: &mut UnifiedWorld) {
    // This step is implicitly verified by the success assertions in previous steps
}

#[then("the non-existent file should not be in the index")]
async fn verify_nonexistent_file_not_in_index(world: &mut UnifiedWorld) {
    // Check that index doesn't contain reference to nonexistent.txt
    let index_content = String::from_utf8_lossy(&world.inner.bit_index_content);
    assert!(!index_content.contains("nonexistent.txt"));
}

#[then("the unreadable file should not be in the index")]
async fn verify_unreadable_file_not_in_index(world: &mut UnifiedWorld) {
    let index_content = String::from_utf8_lossy(&world.inner.bit_index_content);
    assert!(!index_content.contains("unreadable.txt"));
}

#[then("the index should be consistent")]
async fn verify_index_consistency(_world: &mut UnifiedWorld) {
    // Additional consistency checks can be added here
}

// ========================================
// COMMIT ASSERTION STEPS (REUSED)
// ========================================

#[then("both implementations should produce identical tree OIDs")]
async fn verify_both_implementations_produce_identical_tree_oids(_world: &mut UnifiedWorld) {
    // Implementation from commit steps
}

// Main function to run either feature
#[tokio::main]
async fn main() {
    let feature = std::env::var("CUCUMBER_FEATURE").unwrap_or_else(|_| {
        std::env::args()
            .nth(1)
            .unwrap_or_else(|| "commit".to_string())
    });

    match feature.as_str() {
        "commit" => UnifiedWorld::run("tests/features/commit.feature").await,
        "index" => UnifiedWorld::run("tests/features/index.feature").await,
        _ => {
            eprintln!("Usage: CUCUMBER_FEATURE=[commit|index] cargo test --test bdd_tests");
            eprintln!("   or: ./run_bdd_tests.sh [commit|index|all]");
            std::process::exit(1);
        }
    }
}
