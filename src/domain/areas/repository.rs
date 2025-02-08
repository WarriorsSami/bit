use crate::domain::areas::database::Database;
use crate::domain::areas::workspace::Workspace;
use std::cell::{RefCell, RefMut};
use std::path::Path;

pub struct Repository {
    path: Box<Path>,
    writer: RefCell<Box<dyn std::io::Write>>,
    database: Database,
    workspace: Workspace,
}

impl Repository {
    pub fn new(path: &str, writer: Box<dyn std::io::Write>) -> anyhow::Result<Self> {
        let path = Path::new(path).canonicalize()?;

        if !path.exists() {
            std::fs::create_dir_all(&path)?;
        }

        let database = Database::new(path.join(".git").join("objects").into_boxed_path())?;
        let workspace = Workspace::new(path.clone().into_boxed_path())?;

        Ok(Repository {
            path: path.into_boxed_path(),
            writer: RefCell::new(writer),
            database,
            workspace,
        })
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn writer(&self) -> RefMut<Box<dyn std::io::Write>> {
        self.writer.borrow_mut()
    }

    pub fn database(&self) -> &Database {
        &self.database
    }

    pub fn workspace(&self) -> &Workspace {
        &self.workspace
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
}
