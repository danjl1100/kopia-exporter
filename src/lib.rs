//! ## Goals
//!
//! As a self-hosted backup user/operator, there are several aspects of backup that are easy to miss.
//!
//! Step one is to automate the backup, but how do you ensure it stays healthy over time?
//!
//! Monitoring for an unattended backup should verify these key tenets:
//! - New snapshot health
//!     - the newest snapshot should be no older than a specific time threshold
//! - Backup completion status
//!     - verify that backup jobs complete successfully without errors
//! - Data integrity verification
//!     - ensure snapshots are readable and restorable
//! - Repository connectivity
//!     - confirm connection to backup destination is maintained
//! - Performance metrics
//!     - track backup duration and throughput for performance degradation
//! - Remaining space
//!     - `kopia` may not report free space directly, but measuring changes in total space used can signal configuration errors
//! - Pruned snapshots
//!     - The oldest snapshots should be pruned according to retention policy
//! - Pruning health
//!     - Verify that pruning operations complete successfully and maintain expected retention
//!
//! ## Metrics
//!
//! ### New snapshot health
//! - `kopia_snapshot_age_seconds` - Age of newest snapshot in seconds (not present if the snapshots list is empty)
//! - `kopia_snapshot_last_success_timestamp` - Unix timestamp of last successful snapshot (not present if the snapshots list is empty)
//!
//! ### Backup completion status
//! - `kopia_snapshot_errors_total` - Total errors in latest snapshot (not present if the snapshots list is empty)
//! - `kopia_snapshot_ignored_errors_total` - Ignored errors in latest snapshot (not present if the snapshots list is empty)
//!
//! ### Data integrity verification
//! - `kopia_snapshot_failed_files_total` - Number of failed files in latest snapshot (not present if the snapshots list is empty)
//!
//! ### Data quality metrics
//! - `kopia_snapshot_source_parse_errors` - Number of snapshots with unparseable sources (not present if there are no parse errors)
//! - `kopia_snapshot_timestamp_parse_errors_total` - Number of snapshots with unparseable timestamps (not present if there are no parse errors)
//!
//! ### Repository connectivity
//! - `kopia_repository_accessible` - 1 if repository is accessible, 0 otherwise
//!
//! ### Performance metrics
//! - `kopia_snapshot_duration_seconds` - Backup duration (endTime - startTime)
//! - `kopia_snapshot_throughput_bytes_per_second` - Bytes per second throughput
//!
//! ### Remaining space
//! - `kopia_snapshot_total_size_bytes` - Total size of snapshot in bytes (not present if the snapshots list is empty)
//! - `kopia_snapshot_size_change_bytes` - Change in size from previous snapshot (not present if the snapshots list is empty)
//!
//! ### Pruned snapshots
//! - `kopia_snapshots_total` - Total number of snapshots
//! - `kopia_snapshots_by_retention` - Number of snapshots by retention reason (labeled)
//!
//! ### Pruning health
//! - `kopia_retention_policy_violations_total` - Snapshots that should have been pruned but weren't

#![expect(missing_docs)] // TODO after [`KopiaMetrics`] restructure

pub use crate::kopia::*;
use eyre::{Result, eyre};
use std::time::Duration;

pub mod kopia;
pub mod metrics;

#[derive(Clone, Debug)]
pub struct KopiaSnapshots {
    snapshots_map: SourceMap<Vec<Snapshot>>,
    invalid_user_names: std::collections::BTreeMap<String, u32>,
    invalid_hosts: std::collections::BTreeMap<String, u32>,
}

impl KopiaSnapshots {
    /// Creates a new `KopiaSnapshots` from a vector of parsed snapshots.
    ///
    /// # Errors
    ///
    /// Returns an error if `invalid_source_fn` returns an error
    pub fn new_from_snapshots(
        snapshots: Vec<SnapshotJson>,
        invalid_source_fn: impl Fn(SourceStrError) -> eyre::Result<()>,
    ) -> Result<Self> {
        // organize by [`SourceStr`]
        let mut snapshots_map = SourceMap::new();
        let mut invalid_user_names = std::collections::BTreeMap::new();
        let mut invalid_hosts = std::collections::BTreeMap::new();

        for snapshot in snapshots {
            let source_str = match snapshot.source.render() {
                Ok(s) => s,
                Err(e) => {
                    // Track the invalid source
                    if let Some(invalid_user) = e.invalid_user_name() {
                        *invalid_user_names
                            .entry(invalid_user.to_string())
                            .or_insert(0) += 1;
                    }
                    if let Some(invalid_host) = e.invalid_host() {
                        *invalid_hosts.entry(invalid_host.to_string()).or_insert(0) += 1;
                    }

                    // Call the callback for backward compatibility
                    invalid_source_fn(e)?;
                    continue;
                }
            };
            let list: &mut Vec<Snapshot> = snapshots_map.entry(source_str).or_default();
            list.push(snapshot.into());
        }
        Ok(Self {
            snapshots_map,
            invalid_user_names,
            invalid_hosts,
        })
    }

    /// Parses JSON content into a vector of snapshots.
    ///
    /// # Errors
    ///
    /// Returns an error if the JSON content cannot be parsed as snapshot data, or
    /// `invalid_source_fn` returns an error
    pub fn new_parse_json(
        json_content: &str,
        invalid_source_fn: impl Fn(SourceStrError) -> eyre::Result<()>,
    ) -> Result<Self> {
        let snapshots: Vec<SnapshotJson> = serde_json::from_str(json_content)?;
        Self::new_from_snapshots(snapshots, invalid_source_fn)
    }

    /// Executes kopia command to retrieve snapshots and parses the output.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The kopia command fails to execute
    /// - The command returns a non-zero exit code
    /// - The command execution exceeds the specified timeout
    /// - The output cannot be parsed as UTF-8
    /// - The JSON output cannot be parsed as snapshot data
    /// - `invalid_source_fn` returns an error
    pub fn new_from_command(
        kopia_bin: &str,
        timeout: Duration,
        invalid_source_fn: impl Fn(SourceStrError) -> eyre::Result<()>,
    ) -> Result<Self> {
        use std::process::{Command, Stdio};
        use std::time::Instant;

        let mut child = Command::new(kopia_bin)
            .args(["snapshot", "list", "--json"])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;

        let start = Instant::now();
        let poll_interval = Duration::from_millis(50);

        // Poll the child process until it completes or timeout is reached
        loop {
            if let Some(status) = child.try_wait()? {
                // Process completed
                let output = child.wait_with_output()?;

                if !status.success() {
                    let stdout = String::from_utf8_lossy(&output.stdout);
                    let stderr = String::from_utf8_lossy(&output.stderr);
                    return Err(eyre!(
                        "kopia command failed with exit code: {}\nstdout: {}\nstderr: {}",
                        status.code().unwrap_or(-1),
                        stdout,
                        stderr
                    ));
                }

                let stdout = String::from_utf8(output.stdout)?;
                return Self::new_parse_json(&stdout, invalid_source_fn);
            }
            // Process still running, check timeout
            if start.elapsed() >= timeout {
                // Timeout exceeded, kill the process
                let _ = child.kill();
                let _ = child.wait();
                return Err(eyre!(
                    "kopia command timeout after {} seconds",
                    timeout.as_secs_f64()
                ));
            }
            // Sleep briefly before checking again
            std::thread::sleep(poll_interval);
        }
    }

    /// Returns the inner [`SourceMap`]
    #[must_use]
    pub fn into_inner_map(self) -> SourceMap<Vec<Snapshot>> {
        let Self { snapshots_map, .. } = self;
        snapshots_map
    }
}
