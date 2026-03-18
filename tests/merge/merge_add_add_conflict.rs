use crate::common::command::{
    bit_commit, bit_merge, repository_dir, run_bit_command, run_git_command,
};
use crate::common::file::{FileSpec, write_file};
use assert_fs::TempDir;
use rstest::rstest;

/// Test that both branches independently adding the same file is detected as a conflict
///
/// History:
///   A: no new.txt
///   B (master):  new.txt = "ours\n"
///   C (feature): new.txt = "theirs\n"
///
/// Expected: non-zero exit, no base stage (stage 1) for new.txt, stages 2 and 3 present
#[rstest]
fn merge_add_add_conflict(repository_dir: TempDir) -> Result<(), Box<dyn std::error::Error>> {
    let dir = repository_dir;

    run_bit_command(dir.path(), &["init"]).assert().success();

    // Commit A: anchor only
    write_file(FileSpec::new(
        dir.path().join("anchor.txt"),
        "anchor\n".into(),
    ));
    run_bit_command(dir.path(), &["add", "."])
        .assert()
        .success();
    bit_commit(dir.path(), "Commit A").assert().success();

    // Create feature branch at A
    run_bit_command(dir.path(), &["branch", "create", "feature"])
        .assert()
        .success();

    // Commit B on master: add new.txt with "ours"
    write_file(FileSpec::new(dir.path().join("new.txt"), "ours\n".into()));
    run_bit_command(dir.path(), &["add", "."])
        .assert()
        .success();
    bit_commit(dir.path(), "Commit B - add new.txt ours")
        .assert()
        .success();

    // Commit C on feature: add new.txt with "theirs"
    run_bit_command(dir.path(), &["checkout", "feature"])
        .assert()
        .success();
    write_file(FileSpec::new(dir.path().join("new.txt"), "theirs\n".into()));
    run_bit_command(dir.path(), &["add", "."])
        .assert()
        .success();
    bit_commit(dir.path(), "Commit C - add new.txt theirs")
        .assert()
        .success();

    run_bit_command(dir.path(), &["checkout", "master"])
        .assert()
        .success();

    // Merge must fail
    bit_merge(dir.path(), "feature", "add-add conflict")
        .assert()
        .failure();

    // new.txt must still exist (data must not be lost)
    assert!(
        dir.path().join("new.txt").exists(),
        "new.txt should be present in workspace after add/add conflict"
    );

    // Index must have stages 2 and 3, but NOT stage 1 (no common base)
    // Use real `git` because bit has no ls-files command; bit writes a git-compatible index.
    let stage_output = run_git_command(dir.path(), &["ls-files", "--stage"])
        .assert()
        .success();
    let stage_out = String::from_utf8(stage_output.get_output().stdout.clone())?;
    let entries: Vec<&str> = stage_out
        .lines()
        .filter(|l| l.contains("new.txt"))
        .collect();

    // git ls-files --stage format: "<mode> <hash> <stage>\t<path>"
    // Stage is the third space-separated field before the tab.
    let stage_of = |line: &&str| -> Option<u8> {
        line.split('\t')
            .next()
            .and_then(|pre| pre.split_whitespace().nth(2))
            .and_then(|s| s.parse().ok())
    };
    assert!(
        entries.iter().any(|e| stage_of(e) == Some(2)),
        "Missing stage 2 (ours) for new.txt"
    );
    assert!(
        entries.iter().any(|e| stage_of(e) == Some(3)),
        "Missing stage 3 (theirs) for new.txt"
    );
    assert!(
        !entries.iter().any(|e| stage_of(e) == Some(1)),
        "Stage 1 (base) should not exist for add/add conflict — there is no common base"
    );

    Ok(())
}
