use crate::common::command::{bit_commit, run_bit_command};
use crate::common::file::{FileSpec, write_file};
use assert_fs::TempDir;
use pretty_assertions::assert_eq;
use rstest::{fixture, rstest};

/// Create a repository with multiple commits and branches for checkout testing
#[fixture]
pub fn repository_with_branches_and_symbolic_refs() -> TempDir {
    crate::common::redirect_temp_dir();
    let repository_dir = TempDir::new().expect("Failed to create temp dir");

    run_bit_command(repository_dir.path(), &["init"])
        .assert()
        .success();

    // First commit - create initial files
    let file1 = FileSpec::new(
        repository_dir.path().join("file1.txt"),
        "initial content 1".to_string(),
    );
    write_file(file1);

    run_bit_command(repository_dir.path(), &["add", "."])
        .assert()
        .success();

    bit_commit(repository_dir.path(), "Initial commit")
        .assert()
        .success();

    // Create feature branch at first commit
    run_bit_command(repository_dir.path(), &["branch", "feature"])
        .assert()
        .success();

    // Second commit on master
    let file2 = FileSpec::new(
        repository_dir.path().join("file2.txt"),
        "content on master".to_string(),
    );
    write_file(file2);

    run_bit_command(repository_dir.path(), &["add", "."])
        .assert()
        .success();

    bit_commit(repository_dir.path(), "Second commit on master")
        .assert()
        .success();

    // Create develop branch at second commit
    run_bit_command(repository_dir.path(), &["branch", "develop"])
        .assert()
        .success();

    repository_dir
}

#[rstest]
fn checkout_branch_updates_head_to_symbolic_ref(
    repository_with_branches_and_symbolic_refs: TempDir,
) -> Result<(), Box<dyn std::error::Error>> {
    let repository_dir = repository_with_branches_and_symbolic_refs;

    // Initially HEAD should point to master (symbolic ref)
    let head_path = repository_dir.path().join(".git").join("HEAD");
    let initial_head = std::fs::read_to_string(&head_path)?;
    assert!(initial_head.contains("ref: refs/heads/master"));

    // Checkout the feature branch by name
    let output = run_bit_command(repository_dir.path(), &["checkout", "feature"])
        .assert()
        .success();

    // Verify branch switch message is displayed on stderr
    let stderr = String::from_utf8(output.get_output().stderr.clone())?;
    assert!(
        stderr.contains("Switched to branch 'feature'"),
        "Expected branch switch message, got: {}",
        stderr
    );

    // Verify HEAD is now a symbolic ref pointing to feature branch
    let updated_head = std::fs::read_to_string(&head_path)?;
    assert!(
        updated_head.contains("ref: refs/heads/feature"),
        "Expected HEAD to be symbolic ref to feature branch, got: {}",
        updated_head
    );

    // Verify workspace files are updated
    let file1_content = std::fs::read_to_string(repository_dir.path().join("file1.txt"))?;
    assert_eq!(file1_content, "initial content 1");

    // file2 should not exist (added in second commit)
    let file2_path = repository_dir.path().join("file2.txt");
    assert!(!file2_path.exists());

    Ok(())
}

#[rstest]
fn checkout_commit_sha_updates_head_to_detached(
    repository_with_branches_and_symbolic_refs: TempDir,
) -> Result<(), Box<dyn std::error::Error>> {
    let repository_dir = repository_with_branches_and_symbolic_refs;

    // Get the feature branch commit SHA
    let feature_branch_path = repository_dir
        .path()
        .join(".git")
        .join("refs")
        .join("heads")
        .join("feature");
    let feature_commit = std::fs::read_to_string(&feature_branch_path)?;
    let feature_commit_trimmed = feature_commit.trim();

    // Checkout using the commit SHA directly
    let output = run_bit_command(repository_dir.path(), &["checkout", feature_commit_trimmed])
        .assert()
        .success();

    // Verify detachment notice is displayed on stderr
    let stderr = String::from_utf8(output.get_output().stderr.clone())?;
    assert!(
        stderr.contains(&format!("Note: checking out '{}'", feature_commit_trimmed)),
        "Expected detachment notice, got: {}",
        stderr
    );
    assert!(
        stderr.contains("You are in 'detached HEAD' state"),
        "Expected detached HEAD state message, got: {}",
        stderr
    );
    assert!(
        stderr.contains("HEAD is now at"),
        "Expected HEAD position message, got: {}",
        stderr
    );
    assert!(
        stderr.contains("Initial commit"),
        "Expected commit message in output, got: {}",
        stderr
    );

    // Verify HEAD is now a detached HEAD (contains commit SHA, not symbolic ref)
    let head_path = repository_dir.path().join(".git").join("HEAD");
    let updated_head = std::fs::read_to_string(&head_path)?;
    assert!(
        !updated_head.contains("ref:"),
        "Expected HEAD to be detached (no 'ref:'), got: {}",
        updated_head
    );
    assert_eq!(updated_head.trim(), feature_commit_trimmed);

    Ok(())
}

#[rstest]
fn checkout_abbreviated_commit_sha_updates_head_to_detached(
    repository_with_branches_and_symbolic_refs: TempDir,
) -> Result<(), Box<dyn std::error::Error>> {
    let repository_dir = repository_with_branches_and_symbolic_refs;

    // Get the feature branch commit SHA and abbreviate it
    let feature_branch_path = repository_dir
        .path()
        .join(".git")
        .join("refs")
        .join("heads")
        .join("feature");
    let feature_commit = std::fs::read_to_string(&feature_branch_path)?;
    let abbreviated_sha = &feature_commit[..7];

    // Checkout using abbreviated SHA
    let output = run_bit_command(repository_dir.path(), &["checkout", abbreviated_sha])
        .assert()
        .success();

    // Verify detachment notice is displayed on stderr
    let stderr = String::from_utf8(output.get_output().stderr.clone())?;
    assert!(
        stderr.contains(&format!("Note: checking out '{}'", abbreviated_sha)),
        "Expected detachment notice with abbreviated SHA, got: {}",
        stderr
    );
    assert!(
        stderr.contains("You are in 'detached HEAD' state"),
        "Expected detached HEAD state message, got: {}",
        stderr
    );
    assert!(
        stderr.contains("HEAD is now at"),
        "Expected HEAD position message, got: {}",
        stderr
    );

    // Verify HEAD is detached and contains the full commit SHA
    let head_path = repository_dir.path().join(".git").join("HEAD");
    let updated_head = std::fs::read_to_string(&head_path)?;
    assert!(
        !updated_head.contains("ref:"),
        "Expected HEAD to be detached, got: {}",
        updated_head
    );
    assert_eq!(updated_head.trim(), feature_commit.trim());

    Ok(())
}

#[rstest]
fn checkout_branch_then_commit_then_branch_again(
    repository_with_branches_and_symbolic_refs: TempDir,
) -> Result<(), Box<dyn std::error::Error>> {
    let repository_dir = repository_with_branches_and_symbolic_refs;

    let head_path = repository_dir.path().join(".git").join("HEAD");

    // Step 1: Checkout feature branch (symbolic ref)
    let output = run_bit_command(repository_dir.path(), &["checkout", "feature"])
        .assert()
        .success();

    let stderr = String::from_utf8(output.get_output().stderr.clone())?;
    assert!(
        stderr.contains("Switched to branch 'feature'"),
        "Expected branch switch message, got: {}",
        stderr
    );

    let head_after_feature = std::fs::read_to_string(&head_path)?;
    assert!(
        head_after_feature.contains("ref: refs/heads/feature"),
        "Expected symbolic ref to feature, got: {}",
        head_after_feature
    );

    // Step 2: Get commit SHA and checkout detached HEAD
    let feature_branch_path = repository_dir
        .path()
        .join(".git")
        .join("refs")
        .join("heads")
        .join("feature");
    let feature_commit = std::fs::read_to_string(&feature_branch_path)?;

    let output = run_bit_command(repository_dir.path(), &["checkout", feature_commit.trim()])
        .assert()
        .success();

    let stderr = String::from_utf8(output.get_output().stderr.clone())?;
    assert!(
        stderr.contains("Note: checking out"),
        "Expected detachment notice, got: {}",
        stderr
    );
    assert!(
        stderr.contains("You are in 'detached HEAD' state"),
        "Expected detached HEAD state message, got: {}",
        stderr
    );
    assert!(
        stderr.contains("HEAD is now at"),
        "Expected HEAD position message, got: {}",
        stderr
    );

    let head_after_detached = std::fs::read_to_string(&head_path)?;
    assert!(!head_after_detached.contains("ref:"));
    assert_eq!(head_after_detached.trim(), feature_commit.trim());

    // Step 3: Checkout develop branch (symbolic ref again)
    let output = run_bit_command(repository_dir.path(), &["checkout", "develop"])
        .assert()
        .success();

    let stderr = String::from_utf8(output.get_output().stderr.clone())?;
    assert!(
        stderr.contains("Previous HEAD position was"),
        "Expected previous HEAD position message, got: {}",
        stderr
    );
    assert!(
        stderr.contains("Switched to branch 'develop'"),
        "Expected branch switch message, got: {}",
        stderr
    );

    let head_after_develop = std::fs::read_to_string(&head_path)?;
    assert!(
        head_after_develop.contains("ref: refs/heads/develop"),
        "Expected symbolic ref to develop, got: {}",
        head_after_develop
    );

    Ok(())
}

#[rstest]
fn checkout_using_head_parent_from_detached_state(
    repository_with_branches_and_symbolic_refs: TempDir,
) -> Result<(), Box<dyn std::error::Error>> {
    let repository_dir = repository_with_branches_and_symbolic_refs;

    // Get the current HEAD commit SHA (should be at second commit)
    let head_path = repository_dir.path().join(".git").join("HEAD");

    // First, get the develop branch commit (second commit)
    let develop_branch_path = repository_dir
        .path()
        .join(".git")
        .join("refs")
        .join("heads")
        .join("develop");
    let develop_commit = std::fs::read_to_string(&develop_branch_path)?;

    // Checkout develop commit directly (detached HEAD)
    let output = run_bit_command(repository_dir.path(), &["checkout", develop_commit.trim()])
        .assert()
        .success();

    let stderr = String::from_utf8(output.get_output().stderr.clone())?;
    assert!(
        stderr.contains("Note: checking out"),
        "Expected detachment notice, got: {}",
        stderr
    );
    assert!(
        stderr.contains("You are in 'detached HEAD' state"),
        "Expected detached HEAD state message, got: {}",
        stderr
    );

    // Verify we're in detached HEAD state
    let head_content = std::fs::read_to_string(&head_path)?;
    assert!(!head_content.contains("ref:"));

    // Now checkout HEAD^ (parent commit)
    let output = run_bit_command(repository_dir.path(), &["checkout", "HEAD^"])
        .assert()
        .success();

    let stderr = String::from_utf8(output.get_output().stderr.clone())?;
    assert!(
        stderr.contains("Previous HEAD position was"),
        "Expected previous HEAD position message when moving between detached commits, got: {}",
        stderr
    );
    assert!(
        stderr.contains("HEAD is now at"),
        "Expected new HEAD position message, got: {}",
        stderr
    );

    // Verify HEAD is still detached (pointing to parent commit)
    let head_after_parent = std::fs::read_to_string(&head_path)?;
    assert!(
        !head_after_parent.contains("ref:"),
        "Expected detached HEAD after checking out HEAD^, got: {}",
        head_after_parent
    );
    assert_ne!(head_after_parent.trim(), develop_commit.trim());

    // Verify workspace state matches first commit
    let file1_content = std::fs::read_to_string(repository_dir.path().join("file1.txt"))?;
    assert_eq!(file1_content, "initial content 1");

    let file2_path = repository_dir.path().join("file2.txt");
    assert!(!file2_path.exists());

    Ok(())
}

#[rstest]
fn checkout_hierarchical_branch_name(
    repository_with_branches_and_symbolic_refs: TempDir,
) -> Result<(), Box<dyn std::error::Error>> {
    let repository_dir = repository_with_branches_and_symbolic_refs;

    // Create a hierarchical branch name
    run_bit_command(repository_dir.path(), &["branch", "bugfix/issue-123"])
        .assert()
        .success();

    // Checkout the hierarchical branch
    let output = run_bit_command(repository_dir.path(), &["checkout", "bugfix/issue-123"])
        .assert()
        .success();

    // Verify branch switch message is displayed on stderr
    let stderr = String::from_utf8(output.get_output().stderr.clone())?;
    assert!(
        stderr.contains("Switched to branch 'bugfix/issue-123'"),
        "Expected branch switch message, got: {}",
        stderr
    );

    // Verify HEAD is a symbolic ref to the hierarchical branch
    let head_path = repository_dir.path().join(".git").join("HEAD");
    let head_content = std::fs::read_to_string(&head_path)?;
    assert!(
        head_content.contains("ref: refs/heads/bugfix/issue-123"),
        "Expected symbolic ref to bugfix/issue-123, got: {}",
        head_content
    );

    Ok(())
}

#[rstest]
fn checkout_branch_with_same_prefix_as_commit_sha() -> Result<(), Box<dyn std::error::Error>> {
    crate::common::redirect_temp_dir();
    let repository_dir = TempDir::new().expect("Failed to create temp dir");

    run_bit_command(repository_dir.path(), &["init"])
        .assert()
        .success();

    // Create initial commit
    let file1 = FileSpec::new(
        repository_dir.path().join("file1.txt"),
        "content".to_string(),
    );
    write_file(file1);

    run_bit_command(repository_dir.path(), &["add", "."])
        .assert()
        .success();

    bit_commit(repository_dir.path(), "Initial commit")
        .assert()
        .success();

    // Get the commit SHA
    let master_branch_path = repository_dir
        .path()
        .join(".git")
        .join("refs")
        .join("heads")
        .join("master");
    let commit_sha = std::fs::read_to_string(&master_branch_path)?;
    let commit_sha = commit_sha.trim();

    // Create a branch with a name that starts like a commit SHA (e.g., "abc123")
    // This is a contrived test, but demonstrates branch name takes precedence
    let branch_name = format!("{}branch", &commit_sha[..6]);
    run_bit_command(repository_dir.path(), &["branch", &branch_name])
        .assert()
        .success();

    // Checkout the branch by name
    let output = run_bit_command(repository_dir.path(), &["checkout", &branch_name])
        .assert()
        .success();

    // Verify branch switch message is displayed on stderr (not detachment notice)
    let stderr = String::from_utf8(output.get_output().stderr.clone())?;
    assert!(
        stderr.contains(&format!("Switched to branch '{}'", branch_name)),
        "Expected branch switch message, got: {}",
        stderr
    );
    assert!(
        !stderr.contains("detached HEAD"),
        "Should not show detachment notice for branch checkout, got: {}",
        stderr
    );

    // Verify HEAD is a symbolic ref (branch takes precedence over commit prefix)
    let head_path = repository_dir.path().join(".git").join("HEAD");
    let head_content = std::fs::read_to_string(&head_path)?;
    assert!(
        head_content.contains(&format!("ref: refs/heads/{}", branch_name)),
        "Expected symbolic ref to branch, got: {}",
        head_content
    );

    Ok(())
}

#[rstest]
fn checkout_master_after_detached_head() -> Result<(), Box<dyn std::error::Error>> {
    crate::common::redirect_temp_dir();
    let repository_dir = TempDir::new().expect("Failed to create temp dir");

    run_bit_command(repository_dir.path(), &["init"])
        .assert()
        .success();

    // Create two commits
    let file1 = FileSpec::new(repository_dir.path().join("file1.txt"), "first".to_string());
    write_file(file1);

    run_bit_command(repository_dir.path(), &["add", "."])
        .assert()
        .success();

    bit_commit(repository_dir.path(), "First commit")
        .assert()
        .success();

    let file2 = FileSpec::new(
        repository_dir.path().join("file2.txt"),
        "second".to_string(),
    );
    write_file(file2);

    run_bit_command(repository_dir.path(), &["add", "."])
        .assert()
        .success();

    bit_commit(repository_dir.path(), "Second commit")
        .assert()
        .success();

    // Checkout first commit using HEAD^ (detached HEAD)
    let output = run_bit_command(repository_dir.path(), &["checkout", "HEAD^"])
        .assert()
        .success();

    // Verify detachment notice is displayed
    let stderr = String::from_utf8(output.get_output().stderr.clone())?;
    assert!(
        stderr.contains("Note: checking out 'HEAD^'"),
        "Expected detachment notice, got: {}",
        stderr
    );
    assert!(
        stderr.contains("You are in 'detached HEAD' state"),
        "Expected detached HEAD state message, got: {}",
        stderr
    );

    // Verify detached HEAD
    let head_path = repository_dir.path().join(".git").join("HEAD");
    let head_content = std::fs::read_to_string(&head_path)?;
    assert!(!head_content.contains("ref:"));

    // Now checkout master branch by name
    let output = run_bit_command(repository_dir.path(), &["checkout", "master"])
        .assert()
        .success();

    // Verify appropriate messages for returning to a branch from detached HEAD
    let stderr = String::from_utf8(output.get_output().stderr.clone())?;
    assert!(
        stderr.contains("Previous HEAD position was"),
        "Expected previous HEAD position message, got: {}",
        stderr
    );
    assert!(
        stderr.contains("Switched to branch 'master'"),
        "Expected branch switch message, got: {}",
        stderr
    );

    // Verify HEAD is back to symbolic ref
    let head_after_master = std::fs::read_to_string(&head_path)?;
    assert!(
        head_after_master.contains("ref: refs/heads/master"),
        "Expected symbolic ref to master, got: {}",
        head_after_master
    );

    // Verify workspace is back to second commit
    let file2_path = repository_dir.path().join("file2.txt");
    assert!(file2_path.exists());

    Ok(())
}
