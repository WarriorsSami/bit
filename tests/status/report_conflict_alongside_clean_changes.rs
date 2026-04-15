use crate::common::command::{bit_commit, bit_merge, repository_dir, run_bit_command};
use crate::common::file::{FileSpec, write_file};
use assert_fs::TempDir;
use rstest::rstest;

/// Conflicts, staged changes, and untracked files all appear in their respective sections.
///
/// Setup:
///   - f.txt has an edit/edit conflict (appears in "Unmerged paths")
///   - clean.txt is staged with new content (appears in "Changes to be committed")
///   - untracked.txt is not in the index (appears in "Untracked files")
#[rstest]
fn report_conflict_alongside_clean_changes(
    repository_dir: TempDir,
) -> Result<(), Box<dyn std::error::Error>> {
    let dir = repository_dir;

    run_bit_command(dir.path(), &["init"]).assert().success();

    // Initial commit: f.txt and clean.txt
    write_file(FileSpec::new(dir.path().join("f.txt"), "base\n".into()));
    write_file(FileSpec::new(
        dir.path().join("clean.txt"),
        "original\n".into(),
    ));
    run_bit_command(dir.path(), &["add", "."])
        .assert()
        .success();
    bit_commit(dir.path(), "Commit A").assert().success();

    run_bit_command(dir.path(), &["branch", "create", "topic"])
        .assert()
        .success();

    // Modify f.txt on master
    write_file(FileSpec::new(
        dir.path().join("f.txt"),
        "master change\n".into(),
    ));
    run_bit_command(dir.path(), &["add", "."])
        .assert()
        .success();
    bit_commit(dir.path(), "Commit B - master")
        .assert()
        .success();

    // Modify f.txt on topic
    run_bit_command(dir.path(), &["checkout", "topic"])
        .assert()
        .success();
    write_file(FileSpec::new(
        dir.path().join("f.txt"),
        "topic change\n".into(),
    ));
    run_bit_command(dir.path(), &["add", "."])
        .assert()
        .success();
    bit_commit(dir.path(), "Commit C - topic")
        .assert()
        .success();

    run_bit_command(dir.path(), &["checkout", "master"])
        .assert()
        .success();

    // Produce the conflict
    bit_merge(dir.path(), "topic", "merge topic")
        .assert()
        .failure();

    // Stage a clean change to clean.txt
    write_file(FileSpec::new(
        dir.path().join("clean.txt"),
        "staged change\n".into(),
    ));
    run_bit_command(dir.path(), &["add", "clean.txt"])
        .assert()
        .success();

    // Leave untracked.txt untracked
    write_file(FileSpec::new(
        dir.path().join("untracked.txt"),
        "untracked\n".into(),
    ));

    let output = run_bit_command(dir.path(), &["status"]).assert().success();
    let stdout = String::from_utf8(output.get_output().stdout.clone())?;

    assert!(
        stdout.contains("Unmerged paths"),
        "Expected 'Unmerged paths' section in:\n{}",
        stdout
    );
    assert!(
        stdout.contains("f.txt"),
        "Expected conflicted 'f.txt' in output:\n{}",
        stdout
    );
    assert!(
        stdout.contains("Changes to be committed"),
        "Expected 'Changes to be committed' section in:\n{}",
        stdout
    );
    assert!(
        stdout.contains("clean.txt"),
        "Expected 'clean.txt' in staged section:\n{}",
        stdout
    );
    assert!(
        stdout.contains("Untracked files"),
        "Expected 'Untracked files' section in:\n{}",
        stdout
    );
    assert!(
        stdout.contains("untracked.txt"),
        "Expected 'untracked.txt' in untracked section:\n{}",
        stdout
    );

    // f.txt must NOT appear under "Changes to be committed" or "Changes not staged for commit"
    let unmerged_pos = stdout.find("Unmerged paths").unwrap();
    let changes_committed_pos = stdout.find("Changes to be committed");
    if let Some(committed_pos) = changes_committed_pos {
        // The "clean.txt" entry must appear after "Changes to be committed", not before
        let clean_pos = stdout.rfind("clean.txt").unwrap();
        assert!(
            clean_pos > committed_pos,
            "clean.txt should appear after 'Changes to be committed'"
        );
        // f.txt in "Unmerged paths" should appear before "Changes to be committed"
        assert!(
            unmerged_pos < committed_pos,
            "Unmerged paths should appear before Changes to be committed"
        );
    }

    Ok(())
}
