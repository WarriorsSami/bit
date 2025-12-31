use crate::common::command::{
    bit_commit, get_head_commit_sha, init_repository_dir, run_bit_command,
};
use crate::common::file::{FileSpec, delete_path, write_file};
use assert_fs::TempDir;
use rstest::rstest;

#[rstest]
fn show_full_patch_between_commits(
    init_repository_dir: TempDir,
) -> Result<(), Box<dyn std::error::Error>> {
    let repository_dir = init_repository_dir;

    // Get the initial commit SHA (commit with 1.txt, a/2.txt, a/b/3.txt)
    let old_commit_sha = get_head_commit_sha(repository_dir.path())?;

    // Modify file 1.txt
    let file1_modified = FileSpec::new(
        repository_dir.path().join("1.txt"),
        "one modified\nwith new line\n".to_string(),
    );
    write_file(file1_modified);

    // Delete file a/2.txt
    delete_path(repository_dir.path().join("a").join("2.txt").as_path());

    // Modify file a/b/3.txt
    let file3_modified = FileSpec::new(
        repository_dir.path().join("a").join("b").join("3.txt"),
        "three modified".to_string(),
    );
    write_file(file3_modified);

    // Add new file 4.txt
    let file4 = FileSpec::new(
        repository_dir.path().join("4.txt"),
        "four\nnew file\n".to_string(),
    );
    write_file(file4);

    // Stage all changes
    run_bit_command(repository_dir.path(), &["add", "."])
        .assert()
        .success();

    // Create a new commit
    bit_commit(repository_dir.path(), "Second commit with multiple changes")
        .assert()
        .success();

    // Get the new commit SHA
    let new_commit_sha = get_head_commit_sha(repository_dir.path())?;

    // Run diff between two commits to show full patch
    let output = run_bit_command(
        repository_dir.path(),
        &["diff", &old_commit_sha, &new_commit_sha],
    )
    .assert()
    .success();

    let stdout = output.get_output().stdout.clone();
    let actual_output = String::from_utf8(stdout)?;

    // Verify the output contains diff headers for all changed files
    assert!(actual_output.contains("diff --git a/1.txt b/1.txt"));
    assert!(actual_output.contains("--- a/1.txt"));
    assert!(actual_output.contains("+++ b/1.txt"));

    assert!(actual_output.contains("diff --git a/4.txt b/4.txt"));
    assert!(actual_output.contains("--- /dev/null"));
    assert!(actual_output.contains("+++ b/4.txt"));
    assert!(actual_output.contains("new file mode"));

    assert!(actual_output.contains("diff --git a/a/2.txt b/a/2.txt"));
    assert!(actual_output.contains("--- a/a/2.txt"));
    assert!(actual_output.contains("+++ /dev/null"));
    assert!(actual_output.contains("deleted file mode"));

    assert!(actual_output.contains("diff --git a/a/b/3.txt b/a/b/3.txt"));
    assert!(actual_output.contains("--- a/a/b/3.txt"));
    assert!(actual_output.contains("+++ b/a/b/3.txt"));

    // Verify hunks for modified files
    assert!(actual_output.contains("-one"));
    assert!(actual_output.contains("+one modified"));
    assert!(actual_output.contains("+with new line"));

    assert!(actual_output.contains("-three"));
    assert!(actual_output.contains("+three modified"));

    // Verify content for new file
    assert!(actual_output.contains("+four"));
    assert!(actual_output.contains("+new file"));

    // Verify content for deleted file
    assert!(actual_output.contains("-two"));

    Ok(())
}
