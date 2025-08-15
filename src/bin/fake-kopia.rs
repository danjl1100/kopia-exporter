//! A fake kopia binary for testing and development.
//!
//! This binary mimics the behavior of the real kopia CLI tool,
//! providing sample JSON output for snapshot listings and basic
//! repository status commands. Used primarily for testing the
//! kopia-exporter without requiring a real kopia installation.

use clap::{Parser, Subcommand};
use eyre::Result;
use std::fs;

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

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Snapshot { action } => handle_snapshot_command(&action)?,
        Commands::Repository { action } => handle_repository_command(&action),
    }

    Ok(())
}

fn handle_snapshot_command(action: &SnapshotAction) -> Result<()> {
    match action {
        SnapshotAction::List { json } => {
            if *json {
                print_sample_snapshots()
            } else {
                eyre::bail!("fake-kopia only supports --json output for snapshot list");
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

fn print_sample_snapshots() -> Result<()> {
    let sample_path = "src/sample_kopia-snapshot-list.json";
    let content = read_sample_data(sample_path)?;
    print!("{content}");
    Ok(())
}

fn read_sample_data(path: &str) -> Result<String> {
    fs::read_to_string(path).map_err(|e| eyre::eyre!("Could not read sample data from {path}: {e}"))
}
