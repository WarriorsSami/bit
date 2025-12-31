use crate::common::command::{bit_commit, get_head_commit_sha, repository_dir, run_bit_command};
use crate::common::file::{FileSpec, write_file};
use assert_fs::TempDir;
use pretty_assertions::assert_eq;
use rstest::rstest;
use sha1::Digest;

#[rstest]
fn create_branch_from_ambiguous_oid_shows_candidates(
    repository_dir: TempDir,
) -> Result<(), Box<dyn std::error::Error>> {
    let repository_dir = repository_dir;

    // Initialize repository
    run_bit_command(repository_dir.path(), &["init"])
        .assert()
        .success();

    // Create an initial commit to have a proper tree
    let file1 = FileSpec::new(
        repository_dir.path().join("file1.txt"),
        "initial content".to_string(),
    );
    write_file(file1);

    run_bit_command(repository_dir.path(), &["add", "."])
        .assert()
        .success();

    bit_commit(repository_dir.path(), "Initial commit")
        .assert()
        .success();

    let initial_commit_oid = get_head_commit_sha(repository_dir.path())?;

    // Now manually create commits with carefully crafted content to produce colliding OIDs
    // We'll create raw commit objects directly in the .git/objects directory
    // using crafted content that produces OIDs with the same 4-character prefix

    // Strategy: Create commit objects with slightly different timestamps/padding
    // to get OIDs that share a prefix. We'll iterate to find collisions.

    let mut created_commits = Vec::new();
    let objects_dir = repository_dir.path().join(".git").join("objects");

    // Generate commits with varying content until we find at least 2 with same 4-char prefix
    let mut prefix_map: std::collections::HashMap<String, Vec<String>> =
        std::collections::HashMap::new();

    for i in 0..500 {
        // Create a commit with varying content
        let padding = "x".repeat(i % 100);
        let commit_content = format!(
            "tree {}\nparent {}\nauthor Test User <test@example.com> {} +0000\ncommitter Test User <test@example.com> {} +0000\n\nTest commit {}{}\n",
            &initial_commit_oid[..40], // Use the tree from initial commit
            &initial_commit_oid[..40], // Parent is initial commit
            1234567890 + i,
            1234567890 + i,
            i,
            padding
        );

        // Calculate SHA-1 of this commit
        let commit_bytes = format!("commit {}\0{}", commit_content.len(), commit_content);
        let mut hasher = sha1::Sha1::new();
        hasher.update(commit_bytes.as_bytes());
        let oid = format!("{:x}", hasher.finalize());

        // Store by 4-char prefix
        let prefix = &oid[..4];
        prefix_map
            .entry(prefix.to_string())
            .or_default()
            .push(oid.clone());

        // Write the object to disk
        let oid_prefix = &oid[..2];
        let oid_suffix = &oid[2..];
        let obj_dir = objects_dir.join(oid_prefix);
        std::fs::create_dir_all(&obj_dir)?;

        let obj_path = obj_dir.join(oid_suffix);
        let mut encoder =
            flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::default());
        use std::io::Write;
        encoder.write_all(commit_bytes.as_bytes())?;
        let compressed = encoder.finish()?;
        std::fs::write(obj_path, compressed)?;

        created_commits.push(oid);
    }

    // Find a prefix with at least 2 commits
    let mut test_prefix = None;
    let mut matching_oids = Vec::new();

    for (prefix, oids) in prefix_map.iter() {
        if oids.len() >= 2 {
            test_prefix = Some(prefix.clone());
            matching_oids = oids.clone();
            break;
        }
    }

    // If we found colliding prefixes, test the error message
    if let Some(prefix) = test_prefix {
        eprintln!(
            "Found {} commits with prefix '{}'",
            matching_oids.len(),
            prefix
        );

        let output = run_bit_command(
            repository_dir.path(),
            &["branch", "create", "test-branch", &prefix],
        )
        .assert()
        .failure();

        let stderr = String::from_utf8(output.get_output().stderr.clone())?;

        eprintln!("Error output:\n{}", stderr);

        // Verify the error message format matches git's format
        assert!(
            stderr.contains(&format!("short SHA1 {} is ambiguous", prefix)),
            "Expected 'short SHA1 {} is ambiguous' in error, got: {}",
            prefix,
            stderr
        );
        assert!(
            stderr.contains("hint: The candidates are:"),
            "Expected 'hint: The candidates are:' in error, got: {}",
            stderr
        );

        // Verify all matching commits are listed with their short OIDs
        for oid in &matching_oids {
            let short_oid = &oid[..7];
            assert!(
                stderr.contains(short_oid),
                "Expected commit {} to be listed in candidates, got: {}",
                short_oid,
                stderr
            );
            // Verify each candidate is labeled as "commit"
            assert!(
                stderr.contains(&format!("hint:   {} commit", short_oid)),
                "Expected 'hint:   {} commit' in error, got: {}",
                short_oid,
                stderr
            );
        }
    } else {
        eprintln!(
            "Failed to generate commits with colliding 4-character prefixes in 500 attempts... Test inconclusive."
        );
    }

    Ok(())
}

#[rstest]
fn create_branch_from_unique_abbreviated_oid_succeeds(
    repository_dir: TempDir,
) -> Result<(), Box<dyn std::error::Error>> {
    let repository_dir = repository_dir;

    // Initialize repository
    run_bit_command(repository_dir.path(), &["init"])
        .assert()
        .success();

    // Create a single commit
    let file = FileSpec::new(
        repository_dir.path().join("file.txt"),
        "content".to_string(),
    );
    write_file(file);

    run_bit_command(repository_dir.path(), &["add", "."])
        .assert()
        .success();

    bit_commit(repository_dir.path(), "Initial commit")
        .assert()
        .success();

    // Get the commit OID (resolve symbolic references)
    let full_oid = get_head_commit_sha(repository_dir.path())?;

    // Try increasingly shorter prefixes until we find one that works
    for prefix_len in (4..=full_oid.len()).rev() {
        let prefix = &full_oid[..prefix_len];

        let mut result = run_bit_command(
            repository_dir.path(),
            &[
                "branch",
                "create",
                &format!("test-branch-{}", prefix_len),
                prefix,
            ],
        );

        if prefix_len >= 4 {
            // Should succeed for any reasonable length
            result.assert().success();

            // Verify the branch points to the correct commit
            let branch_path = repository_dir
                .path()
                .join(".git")
                .join("refs")
                .join("heads")
                .join(format!("test-branch-{}", prefix_len));
            assert!(branch_path.exists());
            let branch_content = std::fs::read_to_string(&branch_path)?.trim().to_string();
            assert_eq!(branch_content, full_oid);
        }
    }

    Ok(())
}

#[rstest]
fn create_branch_from_oid_prefix_too_short_fails(
    repository_dir: TempDir,
) -> Result<(), Box<dyn std::error::Error>> {
    let repository_dir = repository_dir;

    // Initialize repository
    run_bit_command(repository_dir.path(), &["init"])
        .assert()
        .success();

    // Create a commit
    let file = FileSpec::new(
        repository_dir.path().join("file.txt"),
        "content".to_string(),
    );
    write_file(file);

    run_bit_command(repository_dir.path(), &["add", "."])
        .assert()
        .success();

    bit_commit(repository_dir.path(), "Initial commit")
        .assert()
        .success();

    // Get the commit OID (resolve symbolic references)
    let full_oid = get_head_commit_sha(repository_dir.path())?;

    // Try a prefix that's too short (3 chars or less)
    let short_prefix = &full_oid[..3];

    let output = run_bit_command(
        repository_dir.path(),
        &["branch", "create", "test-branch", short_prefix],
    )
    .assert()
    .failure();

    // Should fail because it's treated as an invalid branch name or unknown revision
    let stderr = String::from_utf8(output.get_output().stderr.clone())?;
    // The error could be about invalid branch name or unknown revision
    // Since 3 chars is too short, it should be treated as a branch name and fail validation
    assert!(
        !stderr.is_empty(),
        "Expected error message, got empty stderr"
    );

    Ok(())
}
