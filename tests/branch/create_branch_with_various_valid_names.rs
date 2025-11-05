use crate::common::command::{init_repository_dir, run_bit_command};
use assert_fs::TempDir;
use pretty_assertions::assert_eq;
use rstest::rstest;

#[rstest]
#[case::simple_alphanumeric("branch123")]
#[case::with_hyphen("feature-branch")]
#[case::with_underscore("feature_branch")]
#[case::mixed("feature-123_test")]
#[case::all_uppercase("FEATURE")]
#[case::all_lowercase("feature")]
#[case::mixed_case("FeatureBranch")]
fn create_branch_with_various_valid_names(
    init_repository_dir: TempDir,
    #[case] branch_name: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let repository_dir = init_repository_dir;

    let head_path = repository_dir.path().join(".git").join("HEAD");
    let head_content = std::fs::read_to_string(&head_path)?;

    // create the branch
    run_bit_command(repository_dir.path(), &["branch", branch_name])
        .assert()
        .success();

    // assert the branch ref exists and its content is equal to the HEAD's content
    let branch_ref_path = repository_dir
        .path()
        .join(".git")
        .join("refs")
        .join("heads")
        .join(branch_name);
    assert!(branch_ref_path.exists());
    let branch_ref_content = std::fs::read_to_string(&branch_ref_path)?;
    assert_eq!(branch_ref_content, head_content);

    Ok(())
}
