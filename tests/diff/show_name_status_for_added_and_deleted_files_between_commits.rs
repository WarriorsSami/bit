use crate::common::command::{
    bit_commit, get_head_commit_sha, init_repository_dir, run_bit_command,
};
use crate::common::file::{FileSpec, delete_path, write_file};
use assert_fs::TempDir;
use rstest::rstest;

#[rstest]
fn show_name_status_for_added_and_deleted_files_between_commits(
    init_repository_dir: TempDir,
) -> Result<(), Box<dyn std::error::Error>> {
    let repository_dir = init_repository_dir;

    // Get the initial commit SHA (commit with 1.txt, a/2.txt, a/b/3.txt)
    let old_commit_sha = get_head_commit_sha(repository_dir.path())?;

    // Delete file 1.txt
    delete_path(repository_dir.path().join("1.txt").as_path());

    // Add file 1.txt back but with different content
    let file1_modified = FileSpec::new(
        repository_dir.path().join("1.txt"),
        "one modified".to_string(),
    );
    write_file(file1_modified);

    // Delete file a/2.txt
    delete_path(repository_dir.path().join("a").join("2.txt").as_path());

    // Add new file 4.txt
    let file4 = FileSpec::new(repository_dir.path().join("4.txt"), "four".to_string());
    write_file(file4);

    // Add new file a/5.txt
    let file5 = FileSpec::new(
        repository_dir.path().join("a").join("5.txt"),
        "five".to_string(),
    );
    write_file(file5);

    // Stage all changes
    run_bit_command(repository_dir.path(), &["add", "."])
        .assert()
        .success();

    // Create a new commit
    bit_commit(
        repository_dir.path(),
        "Second commit with additions and deletions",
    )
    .assert()
    .success();

    // Get the new commit SHA
    let new_commit_sha = get_head_commit_sha(repository_dir.path())?;

    // Run diff with --name-status and --diff-filter=AD
    // This should show only Added and Deleted files (not Modified ones like 1.txt)
    // Output is sorted alphabetically by path, not grouped by status
    let expected_output = "A\t4.txt\nD\ta/2.txt\nA\ta/5.txt\n".to_string();

    let actual_output = run_bit_command(
        repository_dir.path(),
        &[
            "diff",
            &old_commit_sha,
            &new_commit_sha,
            "--name-status",
            "--diff-filter=AD",
        ],
    )
    .assert()
    .success();

    let stdout = actual_output.get_output().stdout.clone();
    let actual_output = String::from_utf8(stdout)?;

    pretty_assertions::assert_eq!(actual_output, expected_output);

    Ok(())
}
