// use crate::common::command::{repository_dir, run_bit_command};
// use crate::common::file::{FileSpec, write_file};
// use assert_fs::TempDir;
// use rstest::rstest;
// use pretty_assertions::assert_eq;
//
// #[rstest]
// fn list_untracked_directories_not_their_contents(
//     repository_dir: TempDir,
// ) -> Result<(), Box<dyn std::error::Error>> {
//     run_bit_command(repository_dir.path(), &["init"])
//         .assert()
//         .success();
//
//     let file = FileSpec::new(repository_dir.path().join("file.txt"), String::new());
//     write_file(file);
//
//     let another_file = FileSpec::new(
//         repository_dir.path().join("dir").join("another_file.txt"),
//         String::new(),
//     );
//     write_file(another_file);
//
//     let expected_output = "?? dir/\n?? file.txt\n".to_string();
//
//     let actual_output = run_bit_command(repository_dir.path(), &["status"])
//         .assert()
//         .success();
//     let stdout = actual_output.get_output().stdout.clone();
//     let actual_output = String::from_utf8(stdout)?;
//
//     assert_eq!(actual_output, expected_output);
//
//     Ok(())
// }
