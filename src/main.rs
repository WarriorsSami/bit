//! # Bit - A Git Implementation in Rust
//!
//! This is a learning project that implements core Git functionality from scratch.
//! It provides a command-line interface for version control operations including:
//! - Repository initialization
//! - File staging and committing
//! - Branch management
//! - Status and diff viewing
//! - Commit history traversal
//!
//! ## Architecture
//!
//! The codebase is organized into three main modules:
//! - `areas`: Core repository components (database, index, refs, workspace)
//! - `artifacts`: Git object types and algorithms (blobs, commits, trees, diff, log)
//! - `commands`: CLI command implementations (plumbing and porcelain)

#![allow(dead_code)]

use crate::artifacts::core::PagerWriter;
use crate::commands::porcelain::log::parse_log_target;
use crate::commands::porcelain::log::{LogOptions, LogRevisionTargets};
use anyhow::Result;
use areas::repository::Repository;
use clap::{Parser, Subcommand, ValueEnum};
use colored::control;
use is_terminal::IsTerminal;
use minus::{Pager, page_all};
use std::path::PathBuf;

// TODO: improve error handling and messages using thiserror
// TODO: improve test harness using snapbox

mod areas;
mod artifacts;
mod commands;

/// Main CLI structure parsed by clap
///
/// This is the top-level command-line interface for the bit version control system.
/// It supports various subcommands for repository management, file operations, and history viewing.
#[derive(Parser)]
#[command(
    name = "bit",
    version = "0.1.0",
    author = "Sami Barbut-Dica",
    about = "A simple git implementation",
    long_about = "This is a simple implementation of git, written in Rust. \
    It is not meant to be a full replacement for git, \
    but rather a learning project to understand how git works under the hood.",
    help_template = r"
{name} {version} - {about}

USAGE:
    {usage}

OPTIONS:
    {all-args}
"
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

/// All available commands in the bit CLI
///
/// Commands are organized into plumbing (low-level) and porcelain (user-facing) operations,
/// similar to Git's command structure.
#[derive(Subcommand)]
enum Commands {
    #[command(
        name = "init",
        about = "Initialize a new repository",
        long_about = "This command initializes a new repository in the current directory or at the specified path."
    )]
    Init {
        #[arg(index = 1, help = "The path to the repository")]
        path: Option<String>,
    },
    #[command(
        name = "hash-object",
        about = "Hash an object and optionally write it to the object database",
        long_about = "This command hashes an object file and can write it to the object database. \
        It requires the path to the file to be specified."
    )]
    HashObject {
        #[arg(
            short,
            long,
            required = false,
            help = "Write the object to the object database"
        )]
        write: bool,
        #[arg(index = 1)]
        file: String,
    },
    #[command(
        name = "ls-tree",
        about = "List the contents of a tree object",
        long_about = "This command lists the contents of a tree object in the repository. \
        It requires the SHA of a tree to be specified."
    )]
    LsTree {
        #[arg(short = 'r', long, help = "Recursively list the tree")]
        recursive: bool,
        #[arg(index = 1, help = "The tree SHA to list")]
        sha: String,
    },
    #[command(
        name = "add",
        about = "Add files or directories to the index",
        long_about = "This command adds the specified files or directories to the index. \
        It requires the paths of the files or directories to be specified."
    )]
    Add {
        #[arg(index = 1, help = "The files or directories to add to the index")]
        paths: Vec<String>,
    },
    #[command(
        name = "commit",
        about = "Create a new commit with the specified message",
        long_about = "This command creates a new commit in the repository with the specified commit message."
    )]
    Commit {
        #[arg(short, long, help = "The commit message")]
        message: String,
    },
    #[command(
        name = "status",
        about = "Show the working tree status",
        long_about = "This command shows the status of the working tree, \
        including staged, unstaged, and untracked files."
    )]
    Status {
        #[arg(
            short,
            long,
            help = "Give the output in a stable, machine-readable format"
        )]
        porcelain: bool,
    },
    #[command(
        name = "diff",
        about = "Show changes between commits, commit and working tree, etc.",
        long_about = "This command shows the differences between various states in the repository."
    )]
    Diff {
        #[arg(
            short,
            long,
            help = "Compare the index to the last commit (HEAD) instead of the working tree"
        )]
        cached: bool,
        #[arg(long, help = "Show only the names and status of changed files")]
        name_status: bool,
        #[arg(
            long,
            help = "Filter the diff output by file status (e.g., A for added, D for deleted, M for modified)"
        )]
        diff_filter: Option<String>,
        #[arg(index = 1, help = "The first commit SHA to compare (optional)")]
        old_revision: Option<String>,
        #[arg(index = 2, help = "The second commit SHA to compare (optional)")]
        new_revision: Option<String>,
    },
    #[command(
        name = "branch",
        about = "Create, list, or delete branches",
        long_about = "This command allows you to create, list, or delete branches in the repository."
    )]
    Branch {
        #[command(subcommand)]
        action: BranchAction,
    },
    #[command(
        name = "checkout",
        about = "Switch branches or restore working tree files",
        long_about = "This command checks out a specified revision, \
        updating the working directory and the index to match the state of that revision."
    )]
    Checkout {
        #[arg(index = 1, help = "The target revision to checkout")]
        target_revision: String,
    },
    #[command(
        name = "log",
        about = "Show commit logs",
        long_about = "This command shows the commit logs of the repository."
    )]
    Log {
        #[arg(
            value_parser = parse_log_target,
            help = "The starting revision(s) to show logs from. \
            They can be revisions, range expressions or excluded revisions."
        )]
        target_revisions: Option<Vec<LogRevisionTargets>>,
        #[arg(
            last = true,
            help = "Show logs only for the specified target files (use after --)"
        )]
        target_files: Option<Vec<PathBuf>>,
        #[arg(long, help = "Show each commit on a single line")]
        oneline: bool,
        #[arg(long, help = "Show abbreviated commit hashes")]
        abbrev_commit: bool,
        #[arg(long, help = "Pretty format for log output")]
        format: Option<CommitDisplayFormat>,
        #[arg(
            long,
            help = "Whether to decorate commit messages with refs (branches, tags, etc.)"
        )]
        decorate: Option<CommitDecoration>,
        #[arg(short, long, help = "Show the full diff of each commit")]
        patch: bool,
    },
    #[command(
        name = "merge",
        about = "Join two or more development histories together",
        long_about = "This command merges the specified revision into the current branch."
    )]
    Merge {
        #[arg(index = 1, help = "The revision to merge into the current branch")]
        target_revision: String,
        #[arg(short, long, help = "The commit message for the merge commit")]
        message: String,
    },
}

/// Format options for displaying commit information
///
/// Controls how commits are rendered in log output.
#[derive(Debug, Clone, Copy, ValueEnum, Default, PartialEq, Eq)]
pub enum CommitDisplayFormat {
    #[value(name = "medium", help = "Medium format")]
    #[default]
    Medium,
    #[value(name = "oneline", help = "One line format")]
    OneLine,
}

/// Decoration options for commit output
///
/// Controls whether and how to display branch/tag references alongside commits.
#[derive(Debug, Clone, Copy, ValueEnum, Default, PartialEq, Eq)]
pub enum CommitDecoration {
    /// No decoration
    None,
    /// Show abbreviated ref names
    #[default]
    Short,
    /// Show full ref names
    Full,
}

/// Branch management subcommands
#[derive(Subcommand)]
enum BranchAction {
    #[command(name = "create", about = "Create a new branch")]
    Create {
        #[arg(index = 1, help = "The name of the branch to create")]
        branch_name: String,
        #[arg(index = 2, help = "Create a new branch from the specified revision")]
        source_refname: Option<String>,
    },
    #[command(name = "delete", about = "Delete one or more branches")]
    Delete {
        #[arg(index = 1, help = "The name(s) of the branch(es) to delete")]
        branch_names: Vec<String>,
        #[arg(short = 'f', long, help = "Force deletion")]
        force: bool,
    },
    #[command(name = "list", about = "List all branches")]
    List {
        #[arg(short = 'v', long, help = "Show more information")]
        verbose: bool,
    },
}

/// Application entry point
///
/// Initializes the async runtime and handles top-level errors.
#[tokio::main]
async fn main() {
    if let Err(err) = run().await {
        eprintln!("Error: {}", err);
        std::process::exit(1);
    }
}

/// Main application logic
///
/// This function:
/// 1. Determines whether to use a pager based on terminal detection
/// 2. Configures colored output appropriately for the output device
/// 3. Parses command-line arguments
/// 4. Dispatches to the appropriate command handler
/// 5. Manages pager lifecycle for commands that benefit from pagination
///
/// # Pager Behavior
///
/// Commands that produce potentially long output (diff, log, branch list) use a pager
/// when output is to a terminal. The pager is skipped when:
/// - Output is redirected to a file or pipe
/// - NO_PAGER environment variable is set
///
/// # Color Handling
///
/// Colors are enabled when using a pager (since final output goes to terminal),
/// and disabled when piping to avoid ANSI escape codes in redirected output.
async fn run() -> Result<()> {
    // Decide whether to use a pager or stdout directly FIRST, before parsing CLI
    // This ensures color settings are applied before any colored output is generated
    let use_pager = std::io::stdout().is_terminal() && std::env::var("NO_PAGER").is_err();

    // Force colored output when using pager (since the final output goes to terminal via pager)
    // We need to set this BEFORE parsing CLI and BEFORE any colored output is generated
    if use_pager {
        control::set_override(true);
    } else if !std::io::stdout().is_terminal() {
        // Disable colors when piping to non-terminal
        control::set_override(false);
    }

    let cli = Cli::parse();
    let pager = Pager::new();

    let stdout_writer = Box::new(std::io::stdout());
    let pager_writer = Box::new(PagerWriter::new(pager.clone()));

    match &cli.command {
        Commands::Init { path } => {
            let mut repository = match path {
                Some(path) => Repository::new(PathBuf::from(path), stdout_writer)?,
                None => {
                    let pwd = std::env::current_dir()?;
                    Repository::new(pwd, stdout_writer)?
                }
            };

            repository.init().await?
        }
        Commands::HashObject { write, file } => {
            let pwd = std::env::current_dir()?;
            let mut repository = Repository::new(pwd, stdout_writer)?;

            repository.hash_object(file, *write)?
        }
        Commands::LsTree { recursive, sha } => {
            let pwd = std::env::current_dir()?;
            let mut repository = Repository::new(pwd, stdout_writer)?;

            repository.ls_tree(sha, *recursive).await?
        }
        Commands::Add { paths } => {
            let pwd = std::env::current_dir()?;
            let mut repository = Repository::new(pwd, stdout_writer)?;

            repository.add(paths).await?
        }
        Commands::Commit { message } => {
            let pwd = std::env::current_dir()?;
            let mut repository = Repository::new(pwd, stdout_writer)?;

            repository.commit(message.as_str()).await?
        }
        Commands::Status { porcelain } => {
            let pwd = std::env::current_dir()?;
            let mut repository = Repository::new(pwd, stdout_writer)?;

            repository.display_status(*porcelain).await?
        }
        Commands::Diff {
            cached,
            name_status,
            diff_filter,
            old_revision,
            new_revision,
        } => {
            let pwd = std::env::current_dir()?;
            let mut repository = Repository::new(
                pwd,
                if use_pager {
                    pager_writer
                } else {
                    stdout_writer
                },
            )?;

            repository
                .diff(
                    *cached,
                    *name_status,
                    diff_filter.as_deref(),
                    old_revision.as_deref(),
                    new_revision.as_deref(),
                )
                .await?;

            if use_pager {
                page_all(pager)?;
            }
        }
        Commands::Branch { action } => {
            let pwd = std::env::current_dir()?;
            let mut repository = Repository::new(
                pwd,
                if use_pager {
                    pager_writer
                } else {
                    stdout_writer
                },
            )?;

            repository.branch(action)?;

            if use_pager {
                page_all(pager)?;
            }
        }
        Commands::Checkout { target_revision } => {
            let pwd = std::env::current_dir()?;
            let mut repository = Repository::new(pwd, stdout_writer)?;

            repository.checkout(target_revision.as_str()).await?
        }
        Commands::Log {
            target_revisions,
            target_files,
            oneline,
            abbrev_commit,
            format,
            decorate,
            patch,
        } => {
            let pwd = std::env::current_dir()?;
            let repository = Repository::new(
                pwd,
                if use_pager {
                    pager_writer
                } else {
                    stdout_writer
                },
            )?;

            repository.log(&LogOptions {
                target_revisions: target_revisions.clone(),
                target_files: target_files.clone(),
                oneline: *oneline,
                abbrev_commit: *abbrev_commit,
                format: (*format).unwrap_or_default(),
                decorate: (*decorate).unwrap_or_default(),
                patch: *patch,
            })?;

            if use_pager {
                page_all(pager)?;
            }
        }
        Commands::Merge {
            target_revision,
            message,
        } => {
            let pwd = std::env::current_dir()?;
            let mut repository = Repository::new(pwd, stdout_writer)?;

            repository
                .merge(target_revision.as_str(), message.as_str())
                .await?
        }
    }

    Ok(())
}
