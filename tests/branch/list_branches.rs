use crate::common::command::{init_repository_dir, run_bit_command};
use assert_fs::TempDir;
use rstest::rstest;

#[rstest]
fn list_branches_with_no_branches(
    init_repository_dir: TempDir,
) -> Result<(), Box<dyn std::error::Error>> {
    let repository_dir = init_repository_dir;

    // List branches (should show nothing or just HEAD)
    let output = run_bit_command(repository_dir.path(), &["branch", "list"])
        .assert()
        .success();

    let stdout = String::from_utf8(output.get_output().stdout.clone())?;
    // With no branches created, output should be the default branch, i.e., master
    assert!(
        stdout.contains("master"),
        "Expected 'master' in output: {}",
        stdout
    );

    Ok(())
}

#[rstest]
fn list_branches_with_single_branch(
    init_repository_dir: TempDir,
) -> Result<(), Box<dyn std::error::Error>> {
    let repository_dir = init_repository_dir;

    // Create a branch
    run_bit_command(repository_dir.path(), &["branch", "create", "main"])
        .assert()
        .success();

    // Checkout the branch to make it current
    run_bit_command(repository_dir.path(), &["checkout", "main"])
        .assert()
        .success();

    // List branches
    let output = run_bit_command(repository_dir.path(), &["branch", "list"])
        .assert()
        .success();

    let stdout = String::from_utf8(output.get_output().stdout.clone())?;
    assert!(
        stdout.contains("main"),
        "Expected 'main' in output: {}",
        stdout
    );

    // Verify the current branch is prefixed with an asterisk
    assert!(
        stdout.contains("* main") || stdout.lines().any(|line| line.trim().starts_with("* main")),
        "Expected current branch 'main' to be prefixed with '*': {}",
        stdout
    );

    Ok(())
}

#[rstest]
fn list_branches_with_multiple_branches(
    init_repository_dir: TempDir,
) -> Result<(), Box<dyn std::error::Error>> {
    let repository_dir = init_repository_dir;

    // Create multiple branches
    run_bit_command(repository_dir.path(), &["branch", "create", "main"])
        .assert()
        .success();
    run_bit_command(repository_dir.path(), &["branch", "create", "develop"])
        .assert()
        .success();
    run_bit_command(repository_dir.path(), &["branch", "create", "feature"])
        .assert()
        .success();

    // Checkout main to make it the current branch
    run_bit_command(repository_dir.path(), &["checkout", "main"])
        .assert()
        .success();

    // List branches
    let output = run_bit_command(repository_dir.path(), &["branch", "list"])
        .assert()
        .success();

    let stdout = String::from_utf8(output.get_output().stdout.clone())?;
    assert!(stdout.contains("main"), "Expected 'main' in output");
    assert!(stdout.contains("develop"), "Expected 'develop' in output");
    assert!(stdout.contains("feature"), "Expected 'feature' in output");

    // Verify the current branch (main) is prefixed with an asterisk
    assert!(
        stdout.contains("* main") || stdout.lines().any(|line| line.trim().starts_with("* main")),
        "Expected current branch 'main' to be prefixed with '*': {}",
        stdout
    );

    // Verify other branches are NOT prefixed with an asterisk (have two spaces instead)
    let lines: Vec<&str> = stdout.lines().collect();
    let develop_line = lines.iter().find(|line| line.contains("develop"));
    let feature_line = lines.iter().find(|line| line.contains("feature"));

    if let Some(line) = develop_line {
        assert!(
            line.trim().starts_with("develop") || line.starts_with("  develop"),
            "Expected 'develop' to NOT be prefixed with '*': {}",
            line
        );
    }

    if let Some(line) = feature_line {
        assert!(
            line.trim().starts_with("feature") || line.starts_with("  feature"),
            "Expected 'feature' to NOT be prefixed with '*': {}",
            line
        );
    }

    Ok(())
}

#[rstest]
fn list_branches_with_hierarchical_names(
    init_repository_dir: TempDir,
) -> Result<(), Box<dyn std::error::Error>> {
    let repository_dir = init_repository_dir;

    // Create branches with hierarchical names
    run_bit_command(
        repository_dir.path(),
        &["branch", "create", "feature/login"],
    )
    .assert()
    .success();
    run_bit_command(
        repository_dir.path(),
        &["branch", "create", "feature/signup"],
    )
    .assert()
    .success();
    run_bit_command(repository_dir.path(), &["branch", "create", "bugfix/auth"])
        .assert()
        .success();

    // Checkout feature/login to make it current
    run_bit_command(repository_dir.path(), &["checkout", "feature/login"])
        .assert()
        .success();

    // List branches
    let output = run_bit_command(repository_dir.path(), &["branch", "list"])
        .assert()
        .success();

    let stdout = String::from_utf8(output.get_output().stdout.clone())?;
    assert!(
        stdout.contains("feature/login"),
        "Expected 'feature/login' in output"
    );
    assert!(
        stdout.contains("feature/signup"),
        "Expected 'feature/signup' in output"
    );
    assert!(
        stdout.contains("bugfix/auth"),
        "Expected 'bugfix/auth' in output"
    );

    // Verify the current branch (feature/login) is prefixed with an asterisk
    assert!(
        stdout.contains("* feature/login")
            || stdout
                .lines()
                .any(|line| line.trim().starts_with("* feature/login")),
        "Expected current branch 'feature/login' to be prefixed with '*': {}",
        stdout
    );

    Ok(())
}

#[rstest]
fn list_branches_sorted_alphabetically(
    init_repository_dir: TempDir,
) -> Result<(), Box<dyn std::error::Error>> {
    let repository_dir = init_repository_dir;

    // Create branches in non-alphabetical order
    run_bit_command(repository_dir.path(), &["branch", "create", "zebra"])
        .assert()
        .success();
    run_bit_command(repository_dir.path(), &["branch", "create", "alpha"])
        .assert()
        .success();
    run_bit_command(repository_dir.path(), &["branch", "create", "beta"])
        .assert()
        .success();

    // Checkout beta to make it current
    run_bit_command(repository_dir.path(), &["checkout", "beta"])
        .assert()
        .success();

    // List branches
    let output = run_bit_command(repository_dir.path(), &["branch", "list"])
        .assert()
        .success();

    let stdout = String::from_utf8(output.get_output().stdout.clone())?;
    let lines: Vec<&str> = stdout.lines().filter(|l| !l.trim().is_empty()).collect();

    // Find positions of branches
    let alpha_pos = lines.iter().position(|l| l.contains("alpha"));
    let beta_pos = lines.iter().position(|l| l.contains("beta"));
    let zebra_pos = lines.iter().position(|l| l.contains("zebra"));

    // Verify alphabetical order
    if let (Some(a), Some(b), Some(z)) = (alpha_pos, beta_pos, zebra_pos) {
        assert!(a < b, "alpha should come before beta");
        assert!(b < z, "beta should come before zebra");
    }

    // Verify the current branch (beta) is prefixed with an asterisk
    assert!(
        stdout.contains("* beta") || stdout.lines().any(|line| line.trim().starts_with("* beta")),
        "Expected current branch 'beta' to be prefixed with '*': {}",
        stdout
    );

    Ok(())
}

#[rstest]
fn list_branches_verbose_shows_commit_info(
    init_repository_dir: TempDir,
) -> Result<(), Box<dyn std::error::Error>> {
    let repository_dir = init_repository_dir;

    // Create a branch
    run_bit_command(repository_dir.path(), &["branch", "create", "main"])
        .assert()
        .success();

    // Checkout the branch to make it current
    run_bit_command(repository_dir.path(), &["checkout", "main"])
        .assert()
        .success();

    // Get the branch OID to verify it's displayed correctly
    let branch_ref_path = repository_dir
        .path()
        .join(".git")
        .join("refs")
        .join("heads")
        .join("main");
    let branch_oid = std::fs::read_to_string(&branch_ref_path)?
        .trim()
        .to_string();
    let abbreviated_oid = &branch_oid[..7];

    // Get the commit object to extract the commit message
    let commit_object_path = repository_dir
        .path()
        .join(".git")
        .join("objects")
        .join(&branch_oid[..2])
        .join(&branch_oid[2..]);

    // Read and parse the commit object to get the commit title
    let commit_content = if commit_object_path.exists() {
        let compressed_data = std::fs::read(&commit_object_path)?;
        let mut decoder = flate2::read::ZlibDecoder::new(&compressed_data[..]);
        let mut decompressed = Vec::new();
        std::io::Read::read_to_end(&mut decoder, &mut decompressed)?;

        // Parse commit object: skip header, find message after blank line
        let content = String::from_utf8_lossy(&decompressed);
        let parts: Vec<&str> = content.splitn(2, '\0').collect();
        if parts.len() == 2 {
            // Find the commit message (after the blank line)
            if let Some(msg_start) = parts[1].find("\n\n") {
                let message = parts[1][msg_start + 2..].trim();
                let commit_title = message.lines().next().unwrap_or("");
                Some(commit_title.to_string())
            } else {
                None
            }
        } else {
            None
        }
    } else {
        None
    };

    // List branches with verbose flag
    let output = run_bit_command(repository_dir.path(), &["branch", "list", "--verbose"])
        .assert()
        .success();

    let stdout = String::from_utf8(output.get_output().stdout.clone())?;
    assert!(stdout.contains("main"), "Expected 'main' in output");

    // Verify the current branch is prefixed with an asterisk
    assert!(
        stdout.contains("* main") || stdout.lines().any(|line| line.trim().starts_with("* main")),
        "Expected current branch 'main' to be prefixed with '*': {}",
        stdout
    );

    // Verify the correct abbreviated OID is displayed
    assert!(
        stdout.contains(abbreviated_oid),
        "Expected abbreviated OID '{}' in verbose output: {}",
        abbreviated_oid,
        stdout
    );

    // Verify the commit title is displayed (if we could extract it)
    if let Some(commit_title) = commit_content {
        assert!(
            stdout.contains(&commit_title),
            "Expected commit title '{}' in verbose output: {}",
            commit_title,
            stdout
        );
    }

    Ok(())
}

#[rstest]
fn list_branches_verbose_short_flag(
    init_repository_dir: TempDir,
) -> Result<(), Box<dyn std::error::Error>> {
    let repository_dir = init_repository_dir;

    // Create a branch
    run_bit_command(repository_dir.path(), &["branch", "create", "main"])
        .assert()
        .success();

    // Checkout the branch to make it current
    run_bit_command(repository_dir.path(), &["checkout", "main"])
        .assert()
        .success();

    // Get the branch OID to verify it's displayed correctly
    let branch_ref_path = repository_dir
        .path()
        .join(".git")
        .join("refs")
        .join("heads")
        .join("main");
    let branch_oid = std::fs::read_to_string(&branch_ref_path)?
        .trim()
        .to_string();
    let abbreviated_oid = &branch_oid[..7];

    // Get the commit object to extract the commit message
    let commit_object_path = repository_dir
        .path()
        .join(".git")
        .join("objects")
        .join(&branch_oid[..2])
        .join(&branch_oid[2..]);

    // Read and parse the commit object to get the commit title
    let commit_content = if commit_object_path.exists() {
        let compressed_data = std::fs::read(&commit_object_path)?;
        let mut decoder = flate2::read::ZlibDecoder::new(&compressed_data[..]);
        let mut decompressed = Vec::new();
        std::io::Read::read_to_end(&mut decoder, &mut decompressed)?;

        // Parse commit object: skip header, find message after blank line
        let content = String::from_utf8_lossy(&decompressed);
        let parts: Vec<&str> = content.splitn(2, '\0').collect();
        if parts.len() == 2 {
            // Find the commit message (after the blank line)
            if let Some(msg_start) = parts[1].find("\n\n") {
                let message = parts[1][msg_start + 2..].trim();
                let commit_title = message.lines().next().unwrap_or("");
                Some(commit_title.to_string())
            } else {
                None
            }
        } else {
            None
        }
    } else {
        None
    };

    // List branches with verbose short flag
    let output = run_bit_command(repository_dir.path(), &["branch", "list", "-v"])
        .assert()
        .success();

    let stdout = String::from_utf8(output.get_output().stdout.clone())?;
    assert!(stdout.contains("main"), "Expected 'main' in output");

    // Verify the current branch is prefixed with an asterisk
    assert!(
        stdout.contains("* main") || stdout.lines().any(|line| line.trim().starts_with("* main")),
        "Expected current branch 'main' to be prefixed with '*': {}",
        stdout
    );

    // Verify the correct abbreviated OID is displayed
    assert!(
        stdout.contains(abbreviated_oid),
        "Expected abbreviated OID '{}' in verbose output: {}",
        abbreviated_oid,
        stdout
    );

    // Verify the commit title is displayed (if we could extract it)
    if let Some(commit_title) = commit_content {
        assert!(
            stdout.contains(&commit_title),
            "Expected commit title '{}' in verbose output: {}",
            commit_title,
            stdout
        );
    }

    Ok(())
}

#[rstest]
fn list_multiple_branches_verbose_shows_all_commit_info(
    init_repository_dir: TempDir,
) -> Result<(), Box<dyn std::error::Error>> {
    let repository_dir = init_repository_dir;

    // Create multiple branches
    run_bit_command(repository_dir.path(), &["branch", "create", "main"])
        .assert()
        .success();
    run_bit_command(repository_dir.path(), &["branch", "create", "develop"])
        .assert()
        .success();
    run_bit_command(repository_dir.path(), &["branch", "create", "feature"])
        .assert()
        .success();

    // Checkout develop to make it current
    run_bit_command(repository_dir.path(), &["checkout", "develop"])
        .assert()
        .success();

    // Helper function to get OID and commit title for a branch
    let get_branch_info = |branch_name: &str| -> Result<
        (String, String, Option<String>),
        Box<dyn std::error::Error>,
    > {
        let branch_ref_path = repository_dir
            .path()
            .join(".git")
            .join("refs")
            .join("heads")
            .join(branch_name);

        let branch_oid = std::fs::read_to_string(&branch_ref_path)?
            .trim()
            .to_string();
        let abbreviated_oid = branch_oid[..7].to_string();

        let commit_object_path = repository_dir
            .path()
            .join(".git")
            .join("objects")
            .join(&branch_oid[..2])
            .join(&branch_oid[2..]);

        let commit_title = if commit_object_path.exists() {
            let compressed_data = std::fs::read(&commit_object_path)?;
            let mut decoder = flate2::read::ZlibDecoder::new(&compressed_data[..]);
            let mut decompressed = Vec::new();
            std::io::Read::read_to_end(&mut decoder, &mut decompressed)?;

            let content = String::from_utf8_lossy(&decompressed);
            let parts: Vec<&str> = content.splitn(2, '\0').collect();
            if parts.len() == 2 {
                if let Some(msg_start) = parts[1].find("\n\n") {
                    let message = parts[1][msg_start + 2..].trim();
                    let title = message.lines().next().unwrap_or("");
                    Some(title.to_string())
                } else {
                    None
                }
            } else {
                None
            }
        } else {
            None
        };

        Ok((branch_oid, abbreviated_oid, commit_title))
    };

    // Get info for all branches
    let (_main_oid, main_abbrev, main_title) = get_branch_info("main")?;
    let (_develop_oid, develop_abbrev, develop_title) = get_branch_info("develop")?;
    let (_feature_oid, feature_abbrev, feature_title) = get_branch_info("feature")?;

    // List branches with verbose flag
    let output = run_bit_command(repository_dir.path(), &["branch", "list", "--verbose"])
        .assert()
        .success();

    let stdout = String::from_utf8(output.get_output().stdout.clone())?;

    // Verify all branches are listed
    assert!(stdout.contains("main"), "Expected 'main' in output");
    assert!(stdout.contains("develop"), "Expected 'develop' in output");
    assert!(stdout.contains("feature"), "Expected 'feature' in output");

    // Verify the current branch (develop) is prefixed with an asterisk
    assert!(
        stdout.contains("* develop")
            || stdout
                .lines()
                .any(|line| line.trim().starts_with("* develop")),
        "Expected current branch 'develop' to be prefixed with '*': {}",
        stdout
    );

    // Verify other branches are NOT prefixed with an asterisk
    let lines: Vec<&str> = stdout.lines().collect();
    let main_line = lines
        .iter()
        .find(|line| line.contains("main") && !line.contains("develop"));
    let feature_line = lines.iter().find(|line| line.contains("feature"));

    if let Some(line) = main_line {
        assert!(
            !line.trim().starts_with("*"),
            "Expected 'main' to NOT be prefixed with '*': {}",
            line
        );
    }

    if let Some(line) = feature_line {
        assert!(
            !line.trim().starts_with("*"),
            "Expected 'feature' to NOT be prefixed with '*': {}",
            line
        );
    }

    // Verify each branch shows its correct abbreviated OID
    assert!(
        stdout.contains(&main_abbrev),
        "Expected main's abbreviated OID '{}' in verbose output: {}",
        main_abbrev,
        stdout
    );
    assert!(
        stdout.contains(&develop_abbrev),
        "Expected develop's abbreviated OID '{}' in verbose output: {}",
        develop_abbrev,
        stdout
    );
    assert!(
        stdout.contains(&feature_abbrev),
        "Expected feature's abbreviated OID '{}' in verbose output: {}",
        feature_abbrev,
        stdout
    );

    // Verify commit titles are displayed for each branch
    if let Some(title) = main_title {
        assert!(
            stdout.contains(&title),
            "Expected main's commit title '{}' in verbose output: {}",
            title,
            stdout
        );
    }

    if let Some(title) = develop_title {
        assert!(
            stdout.contains(&title),
            "Expected develop's commit title '{}' in verbose output: {}",
            title,
            stdout
        );
    }

    if let Some(title) = feature_title {
        assert!(
            stdout.contains(&title),
            "Expected feature's commit title '{}' in verbose output: {}",
            title,
            stdout
        );
    }

    // Verify the output is sorted alphabetically
    let develop_pos = lines.iter().position(|line| line.contains("develop"));
    let feature_pos = lines.iter().position(|line| line.contains("feature"));
    let main_pos = lines
        .iter()
        .position(|line| line.contains("main") && !line.contains("develop"));

    if let (Some(d), Some(f), Some(m)) = (develop_pos, feature_pos, main_pos) {
        assert!(d < f, "develop should come before feature");
        assert!(f < m, "feature should come before main");
    }

    Ok(())
}
