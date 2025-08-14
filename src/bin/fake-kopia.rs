//! TODO document test binary

use clap::{Parser, Subcommand};
use std::fs;
use std::process;

#[derive(Parser)]
#[command(name = "fake-kopia")]
#[command(about = "A stand-in for kopia during development")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Snapshot operations
    Snapshot {
        #[command(subcommand)]
        action: SnapshotAction,
    },
    /// Repository operations
    Repository {
        #[command(subcommand)]
        action: RepositoryAction,
    },
}

#[derive(Subcommand)]
enum SnapshotAction {
    /// List snapshots
    List {
        /// Output in JSON format
        #[arg(long)]
        json: bool,
    },
}

#[derive(Subcommand)]
enum RepositoryAction {
    /// Show repository status
    Status,
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::Snapshot { action } => handle_snapshot_command(&action),
        Commands::Repository { action } => handle_repository_command(&action),
    }
}

fn handle_snapshot_command(action: &SnapshotAction) {
    match action {
        SnapshotAction::List { json } => {
            if *json {
                print_sample_snapshots();
            } else {
                eprintln!("fake-kopia only supports --json output for snapshot list");
                process::exit(1);
            }
        }
    }
}

fn handle_repository_command(action: &RepositoryAction) {
    match action {
        RepositoryAction::Status => {
            println!("Repository status: OK");
            println!("Connected to: fake-repository");
        }
    }
}

fn print_sample_snapshots() {
    let sample_path = "src/sample_kopia-snapshot-list.json";
    // TODO: report error via `eyre`
    match fs::read_to_string(sample_path) {
        Ok(content) => print!("{content}"),
        Err(_) => {
            eprintln!("Error: Could not read sample data from {sample_path}");
            process::exit(1);
        }
    }
}
