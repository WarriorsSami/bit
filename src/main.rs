use anyhow::Result;
use bit::domain::areas::repository::Repository;
use clap::{Parser, Subcommand};

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
",
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

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
        name = "cat-file",
        about = "Print the content of an object",
        long_about = "This command prints the content of an object in the repository. \
        It requires the SHA of the object to be specified."
    )]
    CatFile {
        #[arg(short = 'p', long, help = "The object SHA to print")]
        sha: String,
    },
    #[command(
        name = "hash-object",
        about = "Hash an object and optionally write it to the object database",
        long_about = "This command hashes an object file and can write it to the object database. \
        It requires the path to the file to be specified."
    )]
    HashObject {
        #[arg(short, long, required = false, help = "Write the object to the object database")]
        write: bool,
        #[arg(index = 1)]
        file: String,
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
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match &cli.command {
        Commands::Init { path } => {
            let mut repository = match path {
                Some(path) => Repository::new(path, Box::new(std::io::stdout()))?,
                None => {
                    let pwd = std::env::current_dir()?;
                    Repository::new(&pwd.to_string_lossy(), Box::new(std::io::stdout()))?
                }
            };

            repository.init()?
        }
        Commands::CatFile { sha } => {
            let pwd = std::env::current_dir()?;
            let mut repository =
                Repository::new(&pwd.to_string_lossy(), Box::new(std::io::stdout()))?;

            repository.cat_file(sha)?
        }
        Commands::HashObject { write, file } => {
            let pwd = std::env::current_dir()?;
            let mut repository =
                Repository::new(&pwd.to_string_lossy(), Box::new(std::io::stdout()))?;

            repository.hash_object(file, *write)?
        }
        Commands::Commit { message } => {
            let pwd = std::env::current_dir()?;
            let mut repository =
                Repository::new(&pwd.to_string_lossy(), Box::new(std::io::stdout()))?;

            repository.commit(message.as_str())?
        }
    }

    Ok(())
}
