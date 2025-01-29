use std::path::Path;

pub struct Repository {
    pub(crate) path: Box<Path>,
    pub(crate) writer: Box<dyn std::io::Write>,
}

impl Repository {
    pub fn new(path: &str, writer: Box<dyn std::io::Write>) -> anyhow::Result<Self> {
        let path = Path::new(path).canonicalize()?;
        
        if !path.exists() {
            std::fs::create_dir_all(&path)?;
        }
        
        Ok(Repository {
            path: path.into_boxed_path(),
            writer,
        })
    }

    pub fn git_objects_path(&self) -> Box<Path> {
        self.path.join(".git").join("objects").into_boxed_path()
    }

    pub fn git_refs_path(&self) -> Box<Path> {
        self.path
            .join(".git")
            .join("refs")
            .join("heads")
            .into_boxed_path()
    }

    pub fn git_head_path(&self) -> Box<Path> {
        self.path.join(".git").join("HEAD").into_boxed_path()
    }

    pub fn object_path(&self, object_id: &str) -> Box<Path> {
        self.git_objects_path()
            .join(Path::new(&object_id[..2]))
            .join(Path::new(&object_id[2..]))
            .into_boxed_path()
    }
}
