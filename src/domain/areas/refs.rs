use file_guard::Lock;
use std::io::Write;
use std::path::Path;

pub struct Refs {
    path: Box<Path>,
}

impl Refs {
    pub fn new(path: Box<Path>) -> Self {
        Refs { path }
    }

    pub fn update_head(&self, oid: &str) -> anyhow::Result<()> {
        // open the HEAD file as WRONLY and CREAT to write commit_id to it
        let mut head_file = std::fs::OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(self.head_path())?;
        let mut lock = file_guard::lock(&mut head_file, Lock::Exclusive, 0, 1)?;
        lock.write_all(oid.as_bytes())?;

        Ok(())
    }

    pub fn read_head(&self) -> Option<String> {
        // read HEAD file
        let head = std::fs::read_to_string(self.head_path()).ok()?;

        // return the commit_id if it's not a symbolic reference
        if head.starts_with("ref: ") {
            None
        } else {
            Some(head.trim().to_string())
        }
    }

    pub fn head_path(&self) -> Box<Path> {
        self.path.join("HEAD").into_boxed_path()
    }

    pub fn refs_path(&self) -> Box<Path> {
        self.path.join("refs").into_boxed_path()
    }
}
