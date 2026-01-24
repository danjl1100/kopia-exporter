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

#[derive(Clone, Copy, Debug)]
enum Sleep {
    ForSecs(f64),
    Forever,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    let sleep = std::env::var("FAKE_KOPIA_SLEEP_FOR_SECS")
        .ok()
        .map(|secs| secs.parse().map_or(Sleep::Forever, Sleep::ForSecs));

    // Log each invocation to a file for testing purposes
    log_invocation(sleep)?;

    // Write static test messages to stdout and stderr if requested
    if std::env::var("FAKE_KOPIA_WRITE_TEST_OUTPUT").is_ok() {
        println!("fake-kopia-test-stdout");
        eprintln!("fake-kopia-test-stderr");
    }

    if let Some(sleep) = sleep {
        match sleep {
            Sleep::ForSecs(secs) => {
                std::thread::sleep(std::time::Duration::from_secs_f64(secs));
            }
            Sleep::Forever => loop {
                std::thread::sleep(std::time::Duration::from_secs(1));
            },
        }
    }

    match cli.command {
        Commands::Snapshot { action } => handle_snapshot_command(&action)?,
        Commands::Repository { action } => handle_repository_command(&action),
    }

    Ok(())
}

fn log_invocation(sleep: Option<Sleep>) -> Result<()> {
    if let Ok(log_path) = std::env::var("FAKE_KOPIA_LOG") {
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(log_path)?;
        writeln!(file, "invocation, {sleep:?}")?;
    }
    Ok(())
}

fn handle_snapshot_command(action: &SnapshotAction) -> Result<()> {
    match action {
        SnapshotAction::List { json } => {
            if *json {
                if let Ok(mb_str) = std::env::var("FAKE_KOPIA_LARGE_OUTPUT_MB") {
                    let target_mb: usize = mb_str.parse()?;
                    print_large_snapshots(target_mb)?;
                } else {
                    print_sample_snapshots();
                }
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

fn print_large_snapshots(target_mb: usize) -> Result<()> {
    use std::io::{self, Write};

    let target_bytes = target_mb * 1024 * 1024;

    // Parse the sample snapshots
    let sample_content = include_str!("../sample_kopia-snapshot-list.json");
    let snapshots: Vec<serde_json::Value> = serde_json::from_str(sample_content)?;

    // Ensure we have at least one snapshot to use as a template
    if snapshots.is_empty() {
        eyre::bail!("Sample JSON must contain at least one snapshot");
    }

    // Create a special marker snapshot for the beginning
    let mut marker_start = snapshots[0].clone();
    if let Some(source) = marker_start.get_mut("source")
        && let Some(path) = source.get_mut("path")
    {
        *path = serde_json::json!("/large-output-test-start");
    } else {
        eyre::bail!("Sample JSON snapshot must have source.path field");
    }

    // Create a special marker snapshot for the end
    let mut marker_end = snapshots[0].clone();
    if let Some(source) = marker_end.get_mut("source")
        && let Some(path) = source.get_mut("path")
    {
        *path = serde_json::json!("/large-output-test-end");
    } else {
        eyre::bail!("Sample JSON snapshot must have source.path field");
    }

    // Pre-compute the JSON strings to know exact sizes
    let marker_start_json = serde_json::to_string(&marker_start)?;
    let marker_end_json = serde_json::to_string(&marker_end)?;

    // Calculate exact byte counts for overhead
    let opening_bytes = 1; // "["
    let start_marker_bytes = 1 + 1 + marker_start_json.len() + 1; // "\n " + marker + ","
    let end_marker_bytes = 1 + 1 + marker_end_json.len() + 1 + 1; // "\n " + marker + "\n]"
    let snapshot_overhead_bytes = 1 + 1 + 1; // "\n " + snapshot + ","

    let mut current_bytes = opening_bytes + start_marker_bytes;
    let bytes_reserved_for_end = end_marker_bytes;

    let stdout = io::stdout();
    let mut handle = stdout.lock();

    // Start the JSON array
    write!(handle, "[")?;

    // Write the start marker
    write!(handle, "\n {marker_start_json},")?;

    // Keep adding snapshots until we reach the target size
    let mut index = 0;
    while current_bytes < target_bytes - bytes_reserved_for_end {
        let snapshot = &snapshots[index % snapshots.len()];
        let snapshot_json = serde_json::to_string(snapshot)?;
        let snapshot_total_bytes = snapshot_overhead_bytes + snapshot_json.len();

        // Check if adding this snapshot would exceed our target
        if current_bytes + snapshot_total_bytes + bytes_reserved_for_end > target_bytes {
            break;
        }

        write!(handle, "\n {snapshot_json},")?;
        current_bytes += snapshot_total_bytes;
        index += 1;
    }

    // Write the end marker
    write!(handle, "\n {marker_end_json}\n]")?;

    handle.flush()?;
    Ok(())
}
