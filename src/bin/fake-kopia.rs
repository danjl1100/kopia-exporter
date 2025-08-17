//! A fake kopia binary for testing and development.
//!
//! This binary mimics the behavior of the real kopia CLI tool,
//! providing sample JSON output for snapshot listings and basic
//! repository status commands. Used primarily for testing the
//! kopia-exporter without requiring a real kopia installation.

use clap::{Parser, Subcommand};
use eyre::Result;
use std::fs::OpenOptions;
use std::io::Write;

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

    // Log each invocation to a file for testing purposes
    log_invocation()?;

    match cli.command {
        Commands::Snapshot { action } => handle_snapshot_command(&action)?,
        Commands::Repository { action } => handle_repository_command(&action),
    }

    Ok(())
}

fn log_invocation() -> Result<()> {
    if let Ok(log_path) = std::env::var("FAKE_KOPIA_LOG") {
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(log_path)?;
        // Log with both PID and parent PID to help distinguish test runs
        writeln!(file, "invocation")?;
    }
    Ok(())
}

fn handle_snapshot_command(action: &SnapshotAction) -> Result<()> {
    match action {
        SnapshotAction::List { json } => {
            if *json {
                print_sample_snapshots();
                Ok(())
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

fn print_sample_snapshots() {
    let content = include_str!("../sample_kopia-snapshot-list.json");
    print!("{content}");
}
