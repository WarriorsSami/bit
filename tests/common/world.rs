use assert_cmd::Command;
use assert_fs::TempDir;
use fake::Fake;
use fake::faker::internet::en::FreeEmail;
use fake::faker::lorem::en::{Word, Words};
use fake::faker::name::en::Name;
use std::collections::HashMap;
use std::path::Path;

/// Shared world state for BDD tests
#[derive(Debug, Default)]
pub struct TestWorld {
    pub temp_dir: Option<TempDir>,
    pub file_names: Vec<String>,
    pub author_name: String,
    pub author_email: String,
    pub commit_message: String,
    pub commit_output: String,
    pub bit_tree_oid: String,
    pub git_tree_oid: String,
    pub commit_oid: String,
    pub bit_index_content: Vec<u8>,
    pub git_index_content: Vec<u8>,
    pub file_contents: HashMap<String, String>,
    pub error_output: String,
}

impl TestWorld {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn get_temp_dir(&self) -> &TempDir {
        self.temp_dir
            .as_ref()
            .expect("Temp directory not initialized")
    }

    pub fn get_temp_dir_path(&self) -> &Path {
        self.get_temp_dir().path()
    }

    pub fn create_random_file(&mut self) -> String {
        let file_name = format!("{}.txt", Word().fake::<String>());
        let file_content = Words(5..10).fake::<Vec<String>>().join(" ");
        self.file_contents.insert(file_name.clone(), file_content);
        self.file_names.push(file_name.clone());
        file_name
    }

    pub fn create_random_author_credentials(&mut self) {
        self.author_name = Name().fake::<String>().replace(" ", "_");
        self.author_email = FreeEmail().fake::<String>();
    }

    pub fn create_random_commit_message(&mut self) {
        self.commit_message = Words(5..10).fake::<Vec<String>>().join("\n");
    }

    pub fn run_bit_command(&self, args: &[&str]) -> Command {
        let mut cmd = Command::cargo_bin("bit").expect("Failed to find bit binary");
        cmd.current_dir(self.get_temp_dir_path());
        for arg in args {
            cmd.arg(arg);
        }
        cmd
    }

    pub fn run_git_command(&self, args: &[&str]) -> Command {
        let mut cmd = Command::new("git");
        cmd.current_dir(self.get_temp_dir_path());
        for arg in args {
            cmd.arg(arg);
        }
        cmd
    }
}
