use crate::common::command::{bit_commit, bit_merge, repository_dir, run_bit_command};
use crate::common::file::{FileSpec, write_file};
use assert_fs::TempDir;
use rstest::rstest;
use std::fs;

/// Test merging with complex branching structure
///
/// History:
///         A
///        / \
///       B   C
///       |   |\
///       D   | F
///       |   |/
///       E   G
///       |   |
///       H   I
///       |   |
///       J   K
///
/// Tests merging J and K where the BCA algorithm needs to traverse
/// a complex graph structure
#[rstest]
fn merge_complex_branching(repository_dir: TempDir) -> Result<(), Box<dyn std::error::Error>> {
    let dir = repository_dir;

    // Initialize repository
    run_bit_command(dir.path(), &["init"]).assert().success();

    // Commit A: Base commit
    write_file(FileSpec::new(
        dir.path().join("shared.txt"),
        "A\n".to_string(),
    ));
    run_bit_command(dir.path(), &["add", "."])
        .assert()
        .success();
    bit_commit(dir.path(), "Commit A").assert().success();

    // Create right branch
    run_bit_command(dir.path(), &["branch", "create", "right"])
        .assert()
        .success();

    // Left side: master branch (A -> B -> D -> E -> H -> J)

    // Commit B
    write_file(FileSpec::new(
        dir.path().join("shared.txt"),
        "A\nB\n".to_string(),
    ));
    run_bit_command(dir.path(), &["add", "."])
        .assert()
        .success();
    bit_commit(dir.path(), "Commit B").assert().success();

    // Commit D
    write_file(FileSpec::new(
        dir.path().join("left-only.txt"),
        "D\n".to_string(),
    ));
    run_bit_command(dir.path(), &["add", "."])
        .assert()
        .success();
    bit_commit(dir.path(), "Commit D").assert().success();

    // Commit E
    write_file(FileSpec::new(
        dir.path().join("left-only.txt"),
        "D\nE\n".to_string(),
    ));
    run_bit_command(dir.path(), &["add", "."])
        .assert()
        .success();
    bit_commit(dir.path(), "Commit E").assert().success();

    // Commit H
    write_file(FileSpec::new(
        dir.path().join("left-only.txt"),
        "D\nE\nH\n".to_string(),
    ));
    run_bit_command(dir.path(), &["add", "."])
        .assert()
        .success();
    bit_commit(dir.path(), "Commit H").assert().success();

    // Commit J
    write_file(FileSpec::new(
        dir.path().join("left-only.txt"),
        "D\nE\nH\nJ\n".to_string(),
    ));
    run_bit_command(dir.path(), &["add", "."])
        .assert()
        .success();
    bit_commit(dir.path(), "Commit J").assert().success();

    // Right side: right branch (A -> C -> F -> G -> I -> K)
    run_bit_command(dir.path(), &["checkout", "right"])
        .assert()
        .success();

    // Commit C
    write_file(FileSpec::new(
        dir.path().join("shared.txt"),
        "A\nC\n".to_string(),
    ));
    run_bit_command(dir.path(), &["add", "."])
        .assert()
        .success();
    bit_commit(dir.path(), "Commit C").assert().success();

    // Create temp branch for F
    run_bit_command(dir.path(), &["branch", "create", "temp-f"])
        .assert()
        .success();

    // Commit F on temp branch
    run_bit_command(dir.path(), &["checkout", "temp-f"])
        .assert()
        .success();
    write_file(FileSpec::new(
        dir.path().join("right-only.txt"),
        "F\n".to_string(),
    ));
    run_bit_command(dir.path(), &["add", "."])
        .assert()
        .success();
    bit_commit(dir.path(), "Commit F").assert().success();

    // Back to right, merge F to create G
    run_bit_command(dir.path(), &["checkout", "right"])
        .assert()
        .success();
    bit_merge(dir.path(), "temp-f", "Commit G - merge F")
        .assert()
        .success();

    // Commit I
    write_file(FileSpec::new(
        dir.path().join("right-only.txt"),
        "F\nI\n".to_string(),
    ));
    run_bit_command(dir.path(), &["add", "."])
        .assert()
        .success();
    bit_commit(dir.path(), "Commit I").assert().success();

    // Commit K
    write_file(FileSpec::new(
        dir.path().join("right-only.txt"),
        "F\nI\nK\n".to_string(),
    ));
    run_bit_command(dir.path(), &["add", "."])
        .assert()
        .success();
    bit_commit(dir.path(), "Commit K").assert().success();

    // Final merge: merge right into master (J and K)
    run_bit_command(dir.path(), &["checkout", "master"])
        .assert()
        .success();
    bit_merge(dir.path(), "right", "Merge J and K")
        .assert()
        .success();

    // Verify both branch-specific files exist
    assert!(dir.path().join("shared.txt").exists());
    assert!(dir.path().join("left-only.txt").exists());
    assert!(dir.path().join("right-only.txt").exists());

    // Verify content from left branch
    let left_content =
        fs::read_to_string(dir.path().join("left-only.txt")).expect("Failed to read left-only.txt");
    assert_eq!(left_content, "D\nE\nH\nJ\n");

    // Verify content from right branch
    let right_content = fs::read_to_string(dir.path().join("right-only.txt"))
        .expect("Failed to read right-only.txt");
    assert_eq!(right_content, "F\nI\nK\n");

    Ok(())
}
