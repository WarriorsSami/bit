//! Repository abstraction and coordination
//!
//! This module provides the main `Repository` type that coordinates all repository
//! operations. It acts as a facade over the lower-level components (database, index,
//! workspace, refs) and implements high-level Git commands.
//!
//! ## Architecture
//!
//! The repository maintains references to:
//! - Database: Object storage (blobs, trees, commits)
//! - Index: Staging area for tracking changes
//! - Workspace: Working directory operations
//! - Refs: Branch and reference management
//!
//! ## Thread Safety
//!
//! The index is wrapped in Arc<Mutex<>> to allow safe concurrent access,
//! while other components use interior mutability where needed.

use crate::areas::database::Database;
use crate::areas::index::Index;
use crate::areas::refs::Refs;
use crate::areas::workspace::Workspace;
use crate::artifacts::branch::branch_name::SymRefName;
use crate::artifacts::objects::object_id::ObjectId;
use crate::artifacts::status::status_info::Status;
use std::cell::{Ref, RefCell, RefMut};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::Mutex;

/// Git directory name
const GIT_DIR: &str = ".git";

/// Object database directory name
const DATABASE_DIR: &str = "objects";

/// Index file name
const INDEX_FILE: &str = "index";

/// Git repository
///
/// Coordinates all repository operations and provides access to the database,
/// index, workspace, and refs subsystems. This is the main entry point for
/// all Git operations.
///
/// ## Usage
///
/// ```ignore
/// let repo = Repository::new(PathBuf::from("."), Box::new(stdout()))?;
/// repo.init().await?;
/// repo.add(&["file.txt"]).await?;
/// repo.commit("Initial commit").await?;
/// ```
pub struct Repository {
    /// Repository root path
    path: Box<Path>,
    /// Output writer (stdout or pager)
    writer: RefCell<Box<dyn std::io::Write>>,
    /// Index (staging area) with thread-safe access
    index: Arc<Mutex<Index>>,
    /// Object database
    database: Database,
    /// Working directory
    workspace: Workspace,
    /// Reference manager
    refs: Refs,
    /// Currently checked-out reference (cached)
    current_ref: RefCell<SymRefName>,
    /// Reverse index: OID -> refs that point to it (for decoration)
    reverse_refs: RefCell<HashMap<ObjectId, Vec<SymRefName>>>,
}

impl Repository {
    pub fn new(path: PathBuf, writer: Box<dyn std::io::Write>) -> anyhow::Result<Self> {
        let path = path.canonicalize()?;

        if !path.exists() {
            std::fs::create_dir_all(&path)?;
        }

        let index = Index::new(path.join(GIT_DIR).join(INDEX_FILE).into_boxed_path());
        let database = Database::new(path.join(GIT_DIR).join(DATABASE_DIR).into_boxed_path());
        let workspace = Workspace::new(path.clone().into_boxed_path());
        let refs = Refs::new(path.join(GIT_DIR).into_boxed_path());
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
