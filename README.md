# Bit - Just a lil' bit of Git in Rust

A simple Git implementation written in Rust, based on the book ["Building Your Own Git"](https://shop.jcoglan.com/building-git/) by James Coglan. This project demonstrates the core concepts and internal workings of Git by implementing its fundamental features from scratch.

## Overview

`bit` is an educational Git clone that implements the essential version control functionality of Git. It provides a subset of Git commands including repository initialization, object hashing, file staging, and committing. The project is designed to help understand how Git works under the hood by implementing its core data structures and algorithms.

### Implemented Features

- **Repository initialization** (`bit init`) - Create a new Git repository
- **Object hashing** (`bit hash-object`) - Hash files and store them in the object database
- **Object inspection** (`bit cat-file`) - Display the contents of Git objects
- **File staging** (`bit add`) - Add files to the index (staging area)
- **Committing** (`bit commit`) - Create commits from staged changes

## Architecture

The project follows a clean architecture pattern with clear separation of concerns:

```
src/
├── main.rs              # CLI interface and command routing
├── lib.rs               # Library root
├── commands/            # Command implementations
│   ├── plumbing/        # Low-level Git commands
│   │   ├── cat_file.rs  # Object content display
│   │   └── hash_object.rs # Object hashing
│   └── porcelain/       # High-level user commands
│       ├── add.rs       # File staging
│       ├── commit.rs    # Commit creation
│       └── init.rs      # Repository initialization
└── domain/              # Core domain logic
    ├── areas/           # Git's main areas
    │   ├── database.rs  # Object database
    │   ├── index.rs     # Staging area (index)
    │   ├── refs.rs      # Reference management
    │   ├── repository.rs # Repository operations
    │   └── workspace.rs # Working directory
    └── objects/         # Git object types
        ├── blob.rs      # File content objects
        ├── commit.rs    # Commit objects
        ├── tree.rs      # Directory tree objects
        ├── object_id.rs # SHA-1 identifiers
        └── index_entry.rs # Index entry representation
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

Hash a file:
```bash
./target/release/bit hash-object [-w] <file>
```

View object contents:
```bash
./target/release/bit cat-file -p <sha>
```

Stage files:
```bash
./target/release/bit add <file1> [file2] ...
```

Create a commit:
```bash
./target/release/bit commit -m "commit message"
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

# Test index operations
cargo test index_commands

# Test blob operations
cargo test blob_commands

# Test commit operations  
cargo test commit_commands
```

Run tests with output:
```bash
cargo test -- --nocapture
```

### Test Features

The test suite includes:

- **Integration tests** that verify command-line interface behavior
- **Compatibility tests** that compare `bit` output with actual Git
- **Hexdump utilities** for debugging binary index differences
- **Concurrent operation tests** for index locking behavior
- **Custom assertions** using `pretty_assertions` for better diff visualization

The tests use the `assert_index_eq!` macro to compare binary index files with hexdump output for improved debugging when differences occur.

## Roadmap

### Current Status
- ✅ Repository initialization
- ✅ Object hashing and storage
- ✅ Basic file staging (add command)
- ✅ Simple commit creation
- ✅ Object content inspection
- ✅ Index file format compatibility with Git

### Planned Features

#### Short Term
- [ ] Branch creation and switching
- [ ] Basic merge operations
- [ ] Status command to show working directory state
- [ ] Log command to view commit history
- [ ] Diff command to show changes

#### Medium Term
- [ ] Remote repository support
- [ ] Clone command
- [ ] Push/pull operations
- [ ] Tag management
- [ ] Conflict resolution for merges

#### Long Term
- [ ] Advanced merge strategies
- [ ] Rebase operations
- [ ] Submodule support
- [ ] Hooks system
- [ ] Performance optimizations for large repositories

### Known Limitations

- No networking support (clone, push, pull)
- Limited merge capabilities
- No branch management
- Simplified object packing (no pack files)
- Basic reference handling

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

