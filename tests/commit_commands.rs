use assert_cmd::Command;
use assert_fs::fixture::{FileWriteStr, PathChild};
use assert_fs::prelude::PathCreateDir;
use fake::Fake;
use fake::faker::internet::en::FreeEmail;
use fake::faker::lorem::en::{Word, Words};
use fake::faker::name::en::Name;
use predicates::prelude::predicate;

mod common;

#[test]
fn write_commit_object_successfully_for_flat_project() -> Result<(), Box<dyn std::error::Error>> {
    common::redirect_temp_dir();
    let dir = assert_fs::TempDir::new()?;
    let mut cmd = Command::cargo_bin("bit")?;
    cmd.current_dir(dir.path()).arg("init");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Initialized git directory"));

    // create a few files (random number between 1 and 5) and write random content to them
    let file_count = (1..=5).fake::<usize>();
    let mut file_names = Vec::new();
    for _ in 0..file_count {
        let file_name = format!("{}.txt", Word().fake::<String>());
        let file_path = dir.child(file_name.clone());
        let file_content = Words(5..10).fake::<Vec<String>>().join(" ");
        file_path.write_str(&file_content.clone())?;
        file_names.push(file_name);
    }

    // create fake author config and message
    let author_name = Name().fake::<String>().replace(" ", "_");
    let author_email = FreeEmail().fake::<String>();
    let message = Words(5..10).fake::<Vec<String>>().join("\n");

    // commit the files using bit
    let mut sut = Command::cargo_bin("bit")?;
    sut.current_dir(dir.path())
        .envs(vec![
            ("GIT_AUTHOR_NAME", &author_name),
            ("GIT_AUTHOR_EMAIL", &author_email),
        ])
        .arg("commit")
        .arg("-m")
        .arg(&message);

    // assert that the commit was successful
    let commit_excerpt_raw = sut
        .assert()
        .success()
        .stdout(predicate::str::is_match(
            r"^\[\(root-commit\) [0-9a-f]{40}\] .+$",
        )?)
        .get_output()
        .stdout
        .trim_ascii()
        .to_vec();
    let commit_excerpt = String::from_utf8(commit_excerpt_raw)?;

    // read the HEAD file to get the commit OID
    let head_file_path = dir.child(".git/HEAD").to_path_buf();
    let head_file_content = std::fs::read_to_string(head_file_path)?;

    assert_eq!(head_file_content.len(), 40);
    assert!(head_file_content.chars().all(|c| c.is_ascii_hexdigit()));
    assert!(commit_excerpt.contains(&head_file_content));

    let commit_oid = head_file_content;

    // read the commit object
    let mut sut = Command::cargo_bin("bit")?;
    sut.current_dir(dir.path())
        .arg("cat-file")
        .arg("-p")
        .arg(&commit_oid);

    let output = sut
        .assert()
        .success()
        .stdout(predicate::str::contains(&message))
        .stdout(predicate::str::contains(&author_name))
        .stdout(predicate::str::contains(&author_email))
        .stdout(predicate::str::contains("parent").count(0))
        .get_output()
        .stdout
        .trim_ascii()
        .to_vec();

    // read the tree object
    let output = String::from_utf8(output)?;
    let tree_oid = output
        .lines()
        .next()
        .unwrap()
        .split_whitespace()
        .last()
        .unwrap();

    let mut sut = Command::cargo_bin("bit")?;
    sut.current_dir(dir.path())
        .arg("cat-file")
        .arg("-p")
        .arg(tree_oid);

    sut.assert().success();

    Ok(())
}

#[test]
fn write_commit_object_successfully_for_nested_project() -> Result<(), Box<dyn std::error::Error>> {
    common::redirect_temp_dir();
    let dir = assert_fs::TempDir::new()?;
    let mut cmd = Command::cargo_bin("bit")?;
    cmd.current_dir(dir.path()).arg("init");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Initialized git directory"));

    // create a few files (random number between 1 and 5) and write random content to them
    let file_count = (1..=5).fake::<usize>();
    let mut file_names = Vec::new();
    for _ in 0..file_count {
        let file_name = format!("{}.txt", Word().fake::<String>());
        let file_path = dir.child(file_name.clone());
        let file_content = Words(5..10).fake::<Vec<String>>().join(" ");
        file_path.write_str(&file_content.clone())?;
        file_names.push(file_name);
    }

    // create a few directories (random number between 1 and 5) and create files in them
    let dir_count = (1..=5).fake::<usize>();
    for _ in 0..dir_count {
        let dir_name = Word().fake::<String>();
        let dir_path = dir.child(dir_name.clone());
        dir_path.create_dir_all()?;
        let file_count = (1..=5).fake::<usize>();
        for _ in 0..file_count {
            let file_name = format!("{}.txt", Word().fake::<String>());
            let file_path = dir_path.child(file_name.clone());
            let file_content = Words(5..10).fake::<Vec<String>>().join(" ");
            file_path.write_str(&file_content.clone())?;
            file_names.push(format!("{}/{}", dir_name, file_name));
        }
    }

    let input_tree = common::TreeNode::new(file_names);

    // create fake author config and message
    let author_name = Name().fake::<String>().replace(" ", "_");
    let author_email = FreeEmail().fake::<String>();
    let message = Words(5..10).fake::<Vec<String>>().join("\n");

    // commit the files using bit
    let mut sut = Command::cargo_bin("bit")?;
    sut.current_dir(dir.path())
        .envs(vec![
            ("GIT_AUTHOR_NAME", &author_name),
            ("GIT_AUTHOR_EMAIL", &author_email),
        ])
        .arg("commit")
        .arg("-m")
        .arg(&message);

    // assert that the commit was successful
    let commit_excerpt_raw = sut
        .assert()
        .success()
        .stdout(predicate::str::is_match(
            r"^\[\(root-commit\) [0-9a-f]{40}\] .+$",
        )?)
        .get_output()
        .stdout
        .trim_ascii()
        .to_vec();
    let commit_excerpt = String::from_utf8(commit_excerpt_raw)?;

    // read the HEAD file to get the commit OID
    let head_file_path = dir.child(".git/HEAD").to_path_buf();
    let head_file_content = std::fs::read_to_string(head_file_path)?;

    assert_eq!(head_file_content.len(), 40);
    assert!(head_file_content.chars().all(|c| c.is_ascii_hexdigit()));
    assert!(commit_excerpt.contains(&head_file_content));

    let commit_oid = head_file_content;

    // read the commit object
    let mut sut = Command::cargo_bin("bit")?;
    sut.current_dir(dir.path())
        .arg("cat-file")
        .arg("-p")
        .arg(&commit_oid);

    let output = sut
        .assert()
        .success()
        .stdout(predicate::str::contains(&message))
        .stdout(predicate::str::contains(&author_name))
        .stdout(predicate::str::contains(&author_email))
        .stdout(predicate::str::contains("parent").count(0))
        .get_output()
        .stdout
        .trim_ascii()
        .to_vec();

    // read the tree object
    let output = String::from_utf8(output)?;
    let tree_oid = output
        .lines()
        .next()
        .unwrap()
        .split_whitespace()
        .last()
        .unwrap()
        .to_string();

    let output_tree = common::TreeNode::from_git_object(&dir, tree_oid)?;

    assert_eq!(input_tree, output_tree);

    Ok(())
}
