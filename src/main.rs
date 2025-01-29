use anyhow::Result;
use clap::{Parser, Subcommand};
use bit::domain::git_repository::Repository;

#[derive(Parser)]
#[command(
    name = "bit",
    version = "0.1.0",
    author = "Sami Barbut-Dica",
    about = "A simple git implementation"
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Init {
        #[arg(index = 1)]
        path: Option<String>,
    },
    CatFile {
        #[arg(short = 'p', long)]
        sha: String,
    },
    HashObject {
        #[arg(short, long, required = false)]
        write: bool,
        #[arg(index = 1)]
        file: String,
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
        },
        Commands::CatFile { sha } => {
            let pwd = std::env::current_dir()?;
            let mut repository = Repository::new(&pwd.to_string_lossy(), Box::new(std::io::stdout()))?;

            repository.cat_file(sha)?
        }
        Commands::HashObject { write, file } => {
            let pwd = std::env::current_dir()?;
            let mut repository = Repository::new(&pwd.to_string_lossy(), Box::new(std::io::stdout()))?;

            repository.hash_object(file, *write)?
        },
    }

    Ok(())
}
