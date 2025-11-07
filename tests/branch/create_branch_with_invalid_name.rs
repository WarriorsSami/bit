use crate::common::command::{init_repository_dir, run_bit_command};
use assert_fs::TempDir;
use predicates::prelude::*;
use rstest::rstest;

#[rstest]
#[case::starts_with_dot(".branch")]
#[case::ends_with_lock("branch.lock")]
#[case::consecutive_dots("feature..branch")]
#[case::slash_dot("feature/.branch")]
#[case::starts_with_slash("/branch")]
#[case::ends_with_slash("branch/")]
#[case::at_brace("feature@{0}")]
#[case::asterisk("feature*branch")]
#[case::colon("feature:branch")]
#[case::question_mark("feature?branch")]
#[case::open_bracket("feature[branch")]
#[case::backslash("feature\\branch")]
#[case::caret("feature^branch")]
#[case::tilde("feature~branch")]
#[case::space("feature branch")]
#[case::tab("feature\tbranch")]
#[case::newline("feature\nbranch")]
#[case::carriage_return("feature\rbranch")]
fn create_branch_with_invalid_name(
    init_repository_dir: TempDir,
    #[case] branch_name: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let repository_dir = init_repository_dir;

    // attempt to create a branch with an invalid name
    run_bit_command(repository_dir.path(), &["branch", branch_name])
        .assert()
        .failure()
        .stderr(predicate::str::contains("invalid branch name"));

    // assert the branch ref does NOT exist
    let branch_ref_path = repository_dir
        .path()
        .join(".git")
        .join("refs")
        .join("heads")
        .join(branch_name);
    assert!(!branch_ref_path.exists());

    Ok(())
}

#[rstest]
fn create_branch_with_empty_name(
    init_repository_dir: TempDir,
) -> Result<(), Box<dyn std::error::Error>> {
    let repository_dir = init_repository_dir;

    // attempt to create a branch with an empty name
    run_bit_command(repository_dir.path(), &["branch", ""])
        .assert()
        .failure()
        .stderr(predicate::str::contains("branch name cannot be empty"));

    Ok(())
}
