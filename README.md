# Bit - Just a lil' bit of Git in Rust

A simple Git implementation written in Rust, based on the book ["Building Your Own Git"](https://shop.jcoglan.com/building-git/) by James Coglan. This project demonstrates the core concepts and internal workings of Git by implementing its fundamental features from scratch.

## Overview

`bit` is an educational Git clone that implements the essential version control functionality of Git. It provides a subset of Git commands including repository initialization, object hashing, file staging, and committing. The project is designed to help understand how Git works under the hood by implementing its core data structures and algorithms.

### Implemented Features

- **Repository initialization** (`bit init`) - Create a new Git repository
- **Object hashing** (`bit hash-object`) - Hash files and store them in the object database
- **Tree listing** (`bit ls-tree`) - List the contents of tree objects
- **File staging** (`bit add`) - Add files to the index (staging area)
- **Committing** (`bit commit`) - Create commits from staged changes
- **Status** (`bit status`) - Show the working tree status with staged, unstaged, and untracked files
- **Diff** (`bit diff`) - Show changes between commits, commit and working tree, or index
- **Branch management** (`bit branch`) - Create, list, and delete branches
- **Checkout** (`bit checkout`) - Switch branches or restore working tree files
- **Log** (`bit log`) - Show commit history with various formatting options

## Architecture

The project follows a clean architecture pattern with clear separation of concerns:

```
src/
├── main.rs              # CLI interface and command routing
├── commands/            # Command implementations
│   ├── plumbing/        # Low-level Git commands
│   │   ├── hash_object.rs # Object hashing
│   │   └── ls_tree.rs   # Tree listing
│   └── porcelain/       # High-level user commands
│       ├── add.rs       # File staging
│       ├── commit.rs    # Commit creation
│       ├── status.rs    # Working tree status
│       ├── diff.rs      # Show changes
│       ├── branch.rs    # Branch management
│       ├── checkout.rs  # Switch branches
│       ├── log.rs       # Commit history
│       └── init.rs      # Repository initialization
├── areas/               # Git's main areas
│   ├── database.rs      # Object database
│   ├── index.rs         # Staging area (index)
│   ├── refs.rs          # Reference management
│   ├── repository.rs    # Repository operations
│   └── workspace.rs     # Working directory
└── artifacts/           # Git artifacts and data structures
    ├── objects/         # Git object types
    │   ├── blob.rs      # File content objects
    │   ├── commit.rs    # Commit objects
    │   ├── tree.rs      # Directory tree objects
    │   ├── object_id.rs # SHA-1 identifiers
    │   └── index_entry.rs # Index entry representation
    ├── branch/          # Branch-related structures
    ├── checkout/        # Checkout operations
    ├── database/        # Database structures
    ├── diff/            # Diff algorithms and output
    ├── index/           # Index structures
    └── status/          # Status information
```

### Key Components

- **Repository**: Central coordinator that manages all Git operations
- **Database**: Handles object storage and retrieval using SHA-1 hashing
- **Index**: Manages the staging area where changes are prepared for commits
- **Workspace**: Interfaces with the working directory and file system
- **Objects**: Implements Git's object model (blobs, trees, commits)

## How to Run Locally

### Prerequisites

- Rust 1.88 or later
- Cargo (comes with Rust)

### Installation

1. Clone the repository:
   ```bash
   git clone <repository-url>
   cd bit
   ```

2. Build the project:
   ```bash
   cargo build --release
   ```

3. The binary will be available at `target/release/bit`

### Usage

Initialize a new repository:
```bash
./target/release/bit init [path]
```

Hash a file and optionally write it to the object database:
```bash
./target/release/bit hash-object [-w] <file>
```

List the contents of a tree object:
```bash
./target/release/bit ls-tree [-r] <tree-sha>
```

Stage files for commit:
```bash
./target/release/bit add <file1> [file2] ...
```

Create a commit:
```bash
./target/release/bit commit -m "commit message"
```

Show working tree status:
```bash
./target/release/bit status [--porcelain]
```

Show changes:
```bash
# Show changes in working directory
./target/release/bit diff

# Show changes in staging area (index vs HEAD)
./target/release/bit diff --cached

# Show only file names and status
./target/release/bit diff --name-status

# Compare two commits
./target/release/bit diff <commit-a> <commit-b>

# Filter by file status (A=added, D=deleted, M=modified)
./target/release/bit diff --diff-filter=AD <commit-a> <commit-b>
```

Manage branches:
```bash
# Create a new branch
./target/release/bit branch create <branch-name> [source-revision]

# List all branches
./target/release/bit branch list [-v]

# Delete branches
./target/release/bit branch delete <branch-name> [branch-name2...] [-f]
```

Switch branches or restore files:
```bash
./target/release/bit checkout <target-revision>
```

View commit history:
```bash
# Show commit history
./target/release/bit log

# Show in oneline format
./target/release/bit log --oneline

# Show abbreviated commit hashes
./target/release/bit log --abbrev-commit

# Control decoration display (none, short, full)
./target/release/bit log --decorate=short

# Combine options
./target/release/bit log --oneline --abbrev-commit --decorate=none
```

## How to Run Tests

The project includes comprehensive integration and unit tests that verify compatibility with Git's behavior.

### Test Configuration

**Important**: Before running tests, you need to configure the temporary directory path:

1. Set the `TMPDIR` environment variable or ensure the default `../playground` directory exists:
   ```bash
   mkdir -p ../playground
   ```

2. The tests use a custom temporary directory (`../playground`) instead of the system default to avoid conflicts and provide consistent test environments.

### Running Tests

Run all tests:
```bash
cargo test
```

Run specific test modules:
```bash
# Test repository initialization
cargo test init

# Test add command
cargo test add

# Test commit operations  
cargo test commit

# Test status command
cargo test status

# Test diff operations
cargo test diff

# Test branch operations
cargo test branch

# Test checkout operations
cargo test checkout

# Test log command
cargo test log
```

Run tests with output:
```bash
cargo test -- --nocapture
```

### Test Features

The test suite includes:

- **Integration tests** that verify command-line interface behavior
- **End-to-end tests** for complex workflows (checkout with conflicts, branch operations, etc.)
- **Diff algorithm tests** including hunk generation and various diff scenarios
- **Property-based tests** using `proptest` for revision parsing and other operations
- **Concurrent operation tests** for index locking and consistency
- **Compatibility tests** that verify output matches Git's behavior
- **Custom fixtures** for setting up repository states across multiple commits and branches

## Roadmap

### Current Status
- ✅ Repository initialization
- ✅ Object hashing and storage
- ✅ File staging (add command with comprehensive index management)
- ✅ Commit creation with proper tree generation
- ✅ Tree listing and traversal
- ✅ Status command with staged, unstaged, and untracked files
- ✅ Diff command with hunks, cached mode, and commit comparison
- ✅ Branch creation, listing, and deletion
- ✅ Checkout with conflict detection
- ✅ Log command with multiple format options and decorations
- ✅ Index file format fully compatible with Git
- ✅ Symbolic references (HEAD, branches) 

### Planned Features

#### Short Term
- [ ] Advanced merge operations
- [ ] Conflict resolution during merge
- [ ] Interactive staging (add -p)
- [ ] Stash functionality
- [ ] Cherry-pick operations

#### Medium Term
- [ ] Remote repository support
- [ ] Clone command
- [ ] Push/pull operations
- [ ] Tag management (lightweight and annotated)
- [ ] Git hooks system
- [ ] Reflog for tracking reference changes

#### Long Term
- [ ] Pack files for efficient storage
- [ ] Delta compression
- [ ] Rebase operations (interactive and standard)
- [ ] Submodule support
- [ ] Worktree support
- [ ] Performance optimizations for large repositories
- [ ] Garbage collection

### Known Limitations

- No networking support (clone, push, pull)
- Limited merge capabilities (no three-way merge yet)
- No pack file support (all objects stored loose)
- No reflog tracking
- No git hooks
- No interactive features (interactive add, rebase, etc.)

## Contributing

This project is primarily educational, but contributions are welcome! Please:

1. Fork the repository
2. Create a feature branch
3. Add tests for new functionality
4. Ensure all tests pass
5. Submit a pull request

## License

This project is based on ["Building Your Own Git"](https://shop.jcoglan.com/building-git/) by James Coglan and is intended for educational purposes.

## Acknowledgments

- James Coglan for the excellent "Building Your Own Git" book
- The Git project for the reference implementation
- The Rust community for excellent tooling and libraries

