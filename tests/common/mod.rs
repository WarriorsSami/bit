#![allow(dead_code)]

use assert_cmd::Command;
use assert_fs::TempDir;

const TMPDIR: &str = "../playground";

pub fn redirect_temp_dir() {
    unsafe {
        std::env::set_var("TMPDIR", TMPDIR);
    }
}

#[derive(Debug, Eq, PartialEq)]
pub enum TreeNode {
    File {
        name: String,
    },
    Directory {
        name: String,
        children: Vec<TreeNode>,
    },
}

impl TreeNode {
    pub fn new(file_names: Vec<String>) -> Self {
        // sort the file names so that we can compare the output easily
        let mut file_names = file_names;
        file_names.sort();

        // recursively create the tree structure
        let children = Self::traverse(file_names);

        TreeNode::Directory {
            name: "root".to_string(),
            children,
        }
    }

    fn name(&self) -> &str {
        match self {
            TreeNode::File { name } => name,
            TreeNode::Directory { name, .. } => name,
        }
    }

    pub fn from_git_object(
        repo_dir: &TempDir,
        tree_oid: String,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let output = Self::read_git_object(repo_dir, tree_oid)?;

        let parent_dir = "root".to_string();
        Self::parse_tree(repo_dir, parent_dir, output)
    }

    fn parse_tree(
        repo_dir: &TempDir,
        parent_dir: String,
        output: String,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let children = output
            .lines()
            .flat_map(|line| {
                let mut parts = line.split_whitespace();
                let _mode = parts.next().expect("Missing mode");
                let object_type = parts.next().expect("Missing object type").to_string();
                let oid = parts.next().expect("Missing oid").to_string();
                let name = parts.next().expect("Missing name").to_string();

                assert_eq!(oid.len(), 40, "Invalid oid: {}", oid);
                assert!(
                    oid.chars().all(|c| c.is_ascii_hexdigit()),
                    "Invalid oid: {}",
                    oid
                );

                match object_type.as_str() {
                    "blob" => Ok(TreeNode::File { name }),
                    "tree" => {
                        let child_output =
                            Self::read_git_object(repo_dir, oid).map_err(|e| e.to_string())?;
                        let child = Self::parse_tree(repo_dir, name, child_output)
                            .map_err(|e| e.to_string())?;
                        Ok(child)
                    }
                    _ => Err(format!("Unknown object type: {}", object_type)),
                }
            })
            // group the files/directories by their parent directory
            .fold(Vec::new(), |mut acc, node| {
                match &node {
                    TreeNode::File { .. } => {
                        acc.push(node);
                    }
                    TreeNode::Directory { name, .. } => {
                        let parent = name.split('/').next().unwrap();
                        let parent_idx = acc.iter().position(|n| n.name() == parent);
                        if let Some(idx) = parent_idx {
                            if let TreeNode::Directory { children, .. } = &mut acc[idx] {
                                children.push(node);
                            }
                        }
                    }
                }
                acc
            });

        Ok(TreeNode::Directory {
            name: parent_dir,
            children,
        })
    }

    fn read_git_object(dir: &TempDir, oid: String) -> Result<String, Box<dyn std::error::Error>> {
        let mut sut = Command::cargo_bin("bit")?;
        sut.current_dir(dir.path())
            .arg("cat-file")
            .arg("-p")
            .arg(oid);

        let output = sut.output()?;
        let output = String::from_utf8(output.stdout)?;
        Ok(output)
    }

    fn traverse(file_names: Vec<String>) -> Vec<TreeNode> {
        file_names
            .into_iter()
            .map(|f| {
                if f.contains('/') {
                    let (dir, rest) = f.split_at(f.find('/').unwrap());
                    let rest = rest.strip_prefix('/').unwrap();
                    let children = Self::traverse(vec![rest.to_string()]);
                    TreeNode::Directory {
                        name: dir.to_string(),
                        children,
                    }
                } else {
                    TreeNode::File { name: f }
                }
            })
            // group the files/directories by their parent directory
            .fold(Vec::new(), |mut acc, node| {
                match &node {
                    TreeNode::File { .. } => {
                        acc.push(node);
                    }
                    TreeNode::Directory { name, .. } => {
                        let parent = name.split('/').next().unwrap();
                        let parent_idx = acc.iter().position(|n| n.name() == parent);
                        if let Some(idx) = parent_idx {
                            if let TreeNode::Directory { children, .. } = &mut acc[idx] {
                                children.push(node);
                            }
                        }
                    }
                }
                acc
            })
    }
}
