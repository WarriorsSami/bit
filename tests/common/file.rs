use derive_new::new;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum Descriptor {
    File(FileSpec),
    Directory(DirectorySpec),
}

#[derive(Debug, Clone, Eq, new)]
pub struct FileSpec {
    pub path: PathBuf,
    pub content: String,
}

impl PartialEq for FileSpec {
    fn eq(&self, other: &Self) -> bool {
        self.path.file_name() == other.path.file_name()
    }
}

impl Ord for FileSpec {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.path.file_name().cmp(&other.path.file_name())
    }
}

impl PartialOrd for FileSpec {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

#[derive(Debug, Clone, Eq, new)]
pub struct DirectorySpec {
    pub path: PathBuf,
    pub files: Vec<Descriptor>,
}

impl PartialEq for DirectorySpec {
    fn eq(&self, other: &Self) -> bool {
        self.path.file_name() == other.path.file_name()
    }
}

impl Ord for DirectorySpec {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.path.file_name().cmp(&other.path.file_name())
    }
}

impl PartialOrd for DirectorySpec {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

pub fn write_generated_files(dir: &Path, files_count: usize) -> Vec<FileSpec> {
    use fake::{
        Fake,
        faker::lorem::en::{Word, Words},
    };

    (0..files_count)
        .map(|_| {
            let file_name = format!("{}.txt", Word().fake::<String>());
            let file_path = dir.join(&file_name);
            let file_content = Words(5..10).fake::<Vec<String>>().join(" ");

            let file_spec = FileSpec::new(file_path, file_content);
            write_file(file_spec.clone());

            file_spec
        })
        .collect::<Vec<_>>()
}

pub fn write_file(file_spec: FileSpec) {
    // make sure the parent directory exists
    if let Some(parent) = file_spec.path.parent() {
        std::fs::create_dir_all(parent)
            .unwrap_or_else(|e| panic!("Failed to create directory {:?}: {}", parent, e));
    }

    std::fs::write(&file_spec.path, &file_spec.content)
        .unwrap_or_else(|e| panic!("Failed to write file {:?}: {}", file_spec.path, e));
}

pub fn write_generated_directory(
    dir: &Path,
    files_count: usize,
    subdirs_count: usize,
    depth: usize,
) -> DirectorySpec {
    use fake::{
        Fake,
        faker::lorem::en::{Word, Words},
    };

    let dir_name = format!("dir_{}", Word().fake::<String>());
    let dir_path = dir.join(&dir_name);
    std::fs::create_dir_all(&dir_path)
        .unwrap_or_else(|e| panic!("Failed to create directory {:?}: {}", dir_path, e));

    let mut descriptors = Vec::new();

    // Add files
    for _ in 0..files_count {
        let file_name = format!("{}.txt", Word().fake::<String>());
        let file_path = dir_path.join(&file_name);
        let file_content = Words(5..10).fake::<Vec<String>>().join(" ");

        let file_spec = FileSpec::new(file_path, file_content);
        write_file(file_spec.clone());
        descriptors.push(Descriptor::File(file_spec));
    }

    // Add subdirectories
    if depth > 0 {
        for _ in 0..subdirs_count {
            let subdir_spec =
                write_generated_directory(&dir_path, files_count, subdirs_count, depth - 1);
            descriptors.push(Descriptor::Directory(subdir_spec));
        }
    }

    DirectorySpec::new(dir_path, descriptors)
}

pub fn create_directory(path: &Path) {
    std::fs::create_dir_all(path)
        .unwrap_or_else(|e| panic!("Failed to create directory {:?}: {}", path, e));
}

pub fn list_all_files_statuses(descriptor: &Descriptor) -> Vec<(PathBuf, String)> {
    let mut statuses = Vec::new();

    match descriptor {
        Descriptor::File(file_spec) => {
            statuses.push((file_spec.path.clone(), "??".to_string()));
        }
        Descriptor::Directory(dir_spec) => {
            for desc in &dir_spec.files {
                statuses.extend(list_all_files_statuses(desc));
            }
        }
    }

    // Sort by file name
    statuses.sort_by(|a, b| a.0.file_name().cmp(&b.0.file_name()));

    statuses
}
