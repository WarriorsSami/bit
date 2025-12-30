use crate::areas::database::Database;
use crate::areas::index::Index;
use crate::areas::refs::Refs;
use crate::areas::workspace::Workspace;
use crate::artifacts::branch::branch_name::SymRefName;
use crate::artifacts::objects::object_id::ObjectId;
use crate::artifacts::status::status_info::Status;
use std::cell::{Ref, RefCell, RefMut};
use std::collections::HashMap;
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
    current_ref: RefCell<SymRefName>,
    reverse_refs: RefCell<HashMap<ObjectId, Vec<SymRefName>>>,
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
        let current_ref = refs.current_ref(None)?;

        Ok(Repository {
            path: path.into_boxed_path(),
            writer: RefCell::new(writer),
            index: Arc::new(Mutex::new(index)),
            database,
            workspace,
            refs,
            current_ref: RefCell::new(current_ref),
            reverse_refs: RefCell::new(HashMap::new()),
        })
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn writer(&'_ self) -> RefMut<'_, Box<dyn std::io::Write>> {
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

    pub fn status(&'_ self) -> Status<'_> {
        Status::new(self)
    }

    pub fn current_ref(&self) -> Ref<'_, SymRefName> {
        self.current_ref.borrow()
    }

    pub fn set_current_ref(&self, new_ref: SymRefName) {
        *self.current_ref.borrow_mut() = new_ref;
    }

    pub fn reverse_refs(&self) -> Ref<'_, HashMap<ObjectId, Vec<SymRefName>>> {
        self.reverse_refs.borrow()
    }

    pub fn set_reverse_refs(&self, new_reverse_refs: HashMap<ObjectId, Vec<SymRefName>>) {
        *self.reverse_refs.borrow_mut() = new_reverse_refs;
    }
}
