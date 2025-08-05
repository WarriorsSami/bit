use crate::domain::areas::database::Database;
use crate::domain::areas::index::Index;
use crate::domain::areas::refs::Refs;
use crate::domain::areas::workspace::Workspace;
use std::cell::{RefCell, RefMut};
use std::path::Path;
use std::sync::Arc;
use tokio::sync::Mutex;

pub struct Repository {
    path: Box<Path>,
    writer: RefCell<Box<dyn std::io::Write>>,
    index: Arc<Mutex<Index>>,
    database: Database,
    workspace: Workspace,
    refs: Refs,
}

impl Repository {
    pub fn new(path: &str, writer: Box<dyn std::io::Write>) -> anyhow::Result<Self> {
        let path = Path::new(path).canonicalize()?;

        if !path.exists() {
            std::fs::create_dir_all(&path)?;
        }

        let index = Index::new(path.join(".git").join("index").into_boxed_path());
        let database = Database::new(path.join(".git").join("objects").into_boxed_path());
        let workspace = Workspace::new(path.clone().into_boxed_path());
        let refs = Refs::new(path.join(".git").into_boxed_path());

        Ok(Repository {
            path: path.into_boxed_path(),
            writer: RefCell::new(writer),
            index: Arc::new(Mutex::new(index)),
            database,
            workspace,
            refs,
        })
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn writer(&self) -> RefMut<Box<dyn std::io::Write>> {
        self.writer.borrow_mut()
    }

    pub fn index(&self) -> Arc<Mutex<Index>> {
        self.index.clone()
    }

    pub fn database(&self) -> &Database {
        &self.database
    }

    pub fn workspace(&self) -> &Workspace {
        &self.workspace
    }

    pub fn refs(&self) -> &Refs {
        &self.refs
    }
}
