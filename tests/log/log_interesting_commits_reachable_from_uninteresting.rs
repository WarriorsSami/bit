use crate::common::command::{bit_commit_with_timestamp, repository_dir, run_bit_command};
use crate::common::file::{FileSpec, write_file};
use assert_fs::TempDir;
use rstest::rstest;

#[rstest]
fn log_interesting_commits_reachable_from_uninteresting(
    repository_dir: TempDir,
) -> Result<(), Box<dyn std::error::Error>> {
    // This test exercises the edge case from James Coglan's "Building Git" book (Chapter 16).
    // It validates the timestamp comparison logic in `is_still_interesting()`.
    //
    // Scenario: When all commits have IDENTICAL timestamps, the priority queue processes commits
    // in an arbitrary order (based on OID), bouncing between branches. This can lead to a situation
    // where the output buffer contains interesting commits and the queue contains only uninteresting
    // ones, but we haven't yet discovered that some "interesting" commits are actually reachable
    // from the uninteresting branch.
    //
    // The history (from the book):
    //
    //   A <---- B <---- C <---- D [master]
    //            \
    //             \
    //              E <---- F <---- G <---- H <---- J <---- K [topic]
    //
    // All commits have IDENTICAL timestamps (T0 = 2023-01-01 12:00:00)
    //
    // When processing topic..master (which means ^topic master):
    // - Expected output: D, C (commits on master not reachable from topic)
    // - Should NOT include: B (common ancestor)
    // - With identical timestamps, queue processing order depends on OID
    // - At some point, output might contain [D, C] and queue contains only [G] (uninteresting)
    // - WITHOUT the edge case check, we'd stop because no interesting commits are in the queue
    // - WITH the edge case check, we compare timestamps: oldest_in_output (C) <= newest_in_queue (G)
    //   Since they're equal, we continue and eventually discover B is reachable from K
    //
    // The critical check: If all commits have the same timestamp, we must continue traversal
    // as long as oldest_in_output.timestamp <= newest_in_queue.timestamp (which will be true).

    run_bit_command(repository_dir.path(), &["init"])
        .assert()
        .success();

    // Create all commits with IDENTICAL timestamps to force the edge case
    // where queue processing order is arbitrary (based on OID)
    const TIMESTAMP: &str = "2023-01-01 12:00:00 +0000";

    // Commit A
    let file_a = FileSpec::new(repository_dir.path().join("a.txt"), "commit A".to_string());
    write_file(file_a);
    run_bit_command(repository_dir.path(), &["add", "."])
        .assert()
        .success();
    bit_commit_with_timestamp(repository_dir.path(), "Commit A", TIMESTAMP)
        .assert()
        .success();

    // Commit B - branch point
    let file_b = FileSpec::new(repository_dir.path().join("b.txt"), "commit B".to_string());
    write_file(file_b);
    run_bit_command(repository_dir.path(), &["add", "."])
        .assert()
        .success();
    bit_commit_with_timestamp(repository_dir.path(), "Commit B", TIMESTAMP)
        .assert()
        .success();

    // Create topic branch from B
    run_bit_command(repository_dir.path(), &["branch", "create", "topic"])
        .assert()
        .success();

    // Continue on master: Commit C
    let file_c = FileSpec::new(repository_dir.path().join("c.txt"), "commit C".to_string());
    write_file(file_c);
    run_bit_command(repository_dir.path(), &["add", "."])
        .assert()
        .success();
    bit_commit_with_timestamp(repository_dir.path(), "Commit C", TIMESTAMP)
        .assert()
        .success();

    // Commit D
    let file_d = FileSpec::new(repository_dir.path().join("d.txt"), "commit D".to_string());
    write_file(file_d);
    run_bit_command(repository_dir.path(), &["add", "."])
        .assert()
        .success();
    bit_commit_with_timestamp(repository_dir.path(), "Commit D", TIMESTAMP)
        .assert()
        .success();

    // Switch to topic branch and create commits
    run_bit_command(repository_dir.path(), &["checkout", "topic"])
        .assert()
        .success();

    // Commit E
    let file_e = FileSpec::new(repository_dir.path().join("e.txt"), "commit E".to_string());
    write_file(file_e);
    run_bit_command(repository_dir.path(), &["add", "."])
        .assert()
        .success();
    bit_commit_with_timestamp(repository_dir.path(), "Commit E", TIMESTAMP)
        .assert()
        .success();

    // Commit F
    let file_f = FileSpec::new(repository_dir.path().join("f.txt"), "commit F".to_string());
    write_file(file_f);
    run_bit_command(repository_dir.path(), &["add", "."])
        .assert()
        .success();
    bit_commit_with_timestamp(repository_dir.path(), "Commit F", TIMESTAMP)
        .assert()
        .success();

    // Commit G
    let file_g = FileSpec::new(repository_dir.path().join("g.txt"), "commit G".to_string());
    write_file(file_g);
    run_bit_command(repository_dir.path(), &["add", "."])
        .assert()
        .success();
    bit_commit_with_timestamp(repository_dir.path(), "Commit G", TIMESTAMP)
        .assert()
        .success();

    // Commit H
    let file_h = FileSpec::new(repository_dir.path().join("h.txt"), "commit H".to_string());
    write_file(file_h);
    run_bit_command(repository_dir.path(), &["add", "."])
        .assert()
        .success();
    bit_commit_with_timestamp(repository_dir.path(), "Commit H", TIMESTAMP)
        .assert()
        .success();

    // Commit J
    let file_j = FileSpec::new(repository_dir.path().join("j.txt"), "commit J".to_string());
    write_file(file_j);
    run_bit_command(repository_dir.path(), &["add", "."])
        .assert()
        .success();
    bit_commit_with_timestamp(repository_dir.path(), "Commit J", TIMESTAMP)
        .assert()
        .success();

    // Commit K
    let file_k = FileSpec::new(repository_dir.path().join("k.txt"), "commit K".to_string());
    write_file(file_k);
    run_bit_command(repository_dir.path(), &["add", "."])
        .assert()
        .success();
    bit_commit_with_timestamp(repository_dir.path(), "Commit K", TIMESTAMP)
        .assert()
        .success();

    // Test: topic..master (which means ^topic master)
    // Expected output: D, C (only commits on master not reachable from topic)
    // Should NOT include: B (common ancestor), A, or any topic commits
    //
    // This validates the edge case where:
    // - At some point, output might contain [D, C] and queue might have [G, H, J, K] (all uninteresting)
    // - G.timestamp (T7=17:00) is NEWER than both C (T3=13:00) and D (T4=14:00)
    // - Without the edge case check comparing timestamps, we'd stop too early
    // - The algorithm continues because oldest_in_output.timestamp (C at 13:00) <= newest_in_queue.timestamp (K at 20:00)
    // - This eventually marks B as uninteresting when we discover it's an ancestor of K
    let output = run_bit_command(
        repository_dir.path(),
        &["log", "topic..master", "--decorate=none"],
    )
    .assert()
    .success();

    let stdout = String::from_utf8(output.get_output().stdout.clone())?;

    // Should include only commits on master after the branch point (B)
    assert!(
        stdout.contains("    Commit D"),
        "Expected 'Commit D' in range topic..master"
    );
    assert!(
        stdout.contains("    Commit C"),
        "Expected 'Commit C' in range topic..master"
    );

    // Should NOT include the common ancestor B or its parent A
    assert!(
        !stdout.contains("    Commit B"),
        "Should not include 'Commit B' (common ancestor reachable from topic)"
    );
    assert!(
        !stdout.contains("    Commit A"),
        "Should not include 'Commit A' (ancestor reachable from topic)"
    );

    // Should NOT include any commits from the topic branch
    assert!(
        !stdout.contains("    Commit E"),
        "Should not include 'Commit E' from topic branch"
    );
    assert!(
        !stdout.contains("    Commit F"),
        "Should not include 'Commit F' from topic branch"
    );
    assert!(
        !stdout.contains("    Commit G"),
        "Should not include 'Commit G' from topic branch"
    );
    assert!(
        !stdout.contains("    Commit H"),
        "Should not include 'Commit H' from topic branch"
    );
    assert!(
        !stdout.contains("    Commit J"),
        "Should not include 'Commit J' from topic branch"
    );
    assert!(
        !stdout.contains("    Commit K"),
        "Should not include 'Commit K' from topic branch"
    );

    Ok(())
}
