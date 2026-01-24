//! ## Purpose
//!
//! As a self-hosted backup user/operator, there are several aspects of backup that are easy to miss.
//!
//! Step one is to automate the backup, but how do you ensure it stays healthy over time?
//!
//! Monitoring for an unattended backup should verify these key tenets:
//! - [New snapshot health](Metrics::NEW_SNAPSHOT_HEALTH)
//!     - the newest snapshot should be no older than a specific time threshold
//! - [Backup completion status](Metrics::BACKUP_COMPLETION_STATUS)
//!     - verify that backup jobs complete successfully without errors
//! - [Data integrity verification](Metrics::DATA_INTEGRITY_VERIFICATION)
//!     - ensure snapshots are readable and restorable
// //! - [Repository connectivity](Metrics::REPOSITY_CONNECTIVITY)
// //!     - confirm connection to backup destination is maintained
// //! - [Performance](Metrics::PERFORMANCE)
// //!     - track backup duration and throughput for performance degradation
//! - [Remaining space](Metrics::REMAINING_SPACE)
//!     - `kopia` may not report free space directly, but measuring changes in total space used can signal configuration errors
//! - [Pruned snapshots](Metrics::PRUNED_SNAPSHOTS)
//!     - The oldest snapshots should be pruned according to retention policy
// //! - [Pruning health](Metrics::PRUNING_HEALTH)
// //!     - Verify that pruning operations complete successfully and maintain expected retention
//! - [Data quality](Metrics::DATA_QUALITY)
//!     - Verify that kopia data is valid to be interpreted for metrics generation
//!
//! ## Metrics
//!
//! All available Prometheus metrics are defined in the [`metrics`] module.
//! Each metric is documented in its own module with category and help text.

pub use crate::assert_contains::AssertContains;
pub use crate::kopia::*;
pub use crate::metrics::Metrics;
use eyre::{Result, eyre};
use std::time::Duration;

pub mod kopia;
pub mod metrics;

mod assert_contains;

/// Parsed snapshots list from `kopia`
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

    /// Parses JSON from a reader (streaming).
    ///
    /// This is the primary implementation that streams JSON parsing,
    /// avoiding buffering the entire input in memory.
    ///
    /// # Errors
    ///
    /// Returns an error if the JSON content cannot be parsed as snapshot data, or
    /// `invalid_source_fn` returns an error
    pub fn new_from_reader(
        reader: impl std::io::Read,
        invalid_source_fn: impl Fn(SourceStrError) -> eyre::Result<()>,
    ) -> Result<Self> {
        let snapshots: Vec<SnapshotJson> = serde_json::from_reader(reader)?;
        Self::new_from_snapshots(snapshots, invalid_source_fn)
    }

    /// Parses JSON content from a string.
    ///
    /// This is a convenience wrapper around [`Self::new_from_reader`] for tests
    /// and cases where the JSON is already in memory as a string.
    ///
    /// # Errors
    ///
    /// Returns an error if the JSON content cannot be parsed as snapshot data, or
    /// `invalid_source_fn` returns an error
    pub fn new_parse_json(
        json_content: &str,
        invalid_source_fn: impl Fn(SourceStrError) -> eyre::Result<()>,
    ) -> Result<Self> {
        Self::new_from_reader(std::io::Cursor::new(json_content), invalid_source_fn)
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
        invalid_source_fn: impl Fn(SourceStrError) -> eyre::Result<()> + Send + 'static,
    ) -> Result<Self> {
        use std::io::Read;
        use std::process::{Command, Stdio};
        use std::sync::mpsc;
        use std::time::Instant;

        let mut child = Command::new(kopia_bin)
            .args(["snapshot", "list", "--json"])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;

        // Take ownership of stdout and stderr pipes
        let stdout_pipe = child
            .stdout
            .take()
            .ok_or_else(|| eyre!("Failed to capture stdout"))?;
        let stderr_pipe = child
            .stderr
            .take()
            .ok_or_else(|| eyre!("Failed to capture stderr"))?;

        // Spawn thread to parse JSON directly from stdout stream
        // This avoids buffering the entire JSON in memory before parsing
        let (result_tx, result_rx) = mpsc::channel();
        std::thread::spawn(move || {
            let result = Self::new_from_reader(stdout_pipe, invalid_source_fn);
            let _ = result_tx.send(result);
        });

        // Spawn thread to read stderr (to prevent blocking)
        let (stderr_tx, stderr_rx) = mpsc::channel();
        std::thread::spawn(move || {
            let mut stderr_pipe = stderr_pipe;
            let mut buffer = Vec::new();
            let _ = stderr_pipe.read_to_end(&mut buffer);
            let _ = stderr_tx.send(buffer);
        });

        let start = Instant::now();
        let poll_interval = Duration::from_millis(50);

        // Poll the child process until it completes or timeout is reached
        loop {
            if let Some(status) = child.try_wait()? {
                // Process completed - get results from threads
                let parse_result = result_rx
                    .recv()
                    .map_err(|_| eyre!("Failed to receive parse result from thread"))?;
                let stderr_buffer = stderr_rx
                    .recv()
                    .map_err(|_| eyre!("Failed to receive stderr from thread"))?;

                if !status.success() {
                    let stderr = String::from_utf8_lossy(&stderr_buffer);
                    return Err(eyre!(
                        "kopia command failed with exit code: {}\nstderr: {}",
                        status.code().unwrap_or(-1),
                        stderr
                    ));
                }

                // Return the parse result, which may contain JSON parsing errors
                return parse_result;
            }

            // Check timeout
            if start.elapsed() >= timeout {
                // Timeout exceeded, kill the process
                let _ = child.kill();
                let _ = child.wait();

                let seconds = timeout.as_secs_f64();

                // Try to get whatever output the threads have captured
                let Ok(stderr_buffer) = stderr_rx.recv() else {
                    return Err(eyre!(
                        "kopia command timeout after {seconds} seconds\n<stderr is unknown>",
                    ));
                };
                let stderr = String::from_utf8_lossy(&stderr_buffer);

                // Note: We can't easily get partial stdout since it's being consumed by the parser
                return Err(eyre!(
                    "kopia command timeout after {seconds} seconds\nstderr: {stderr}",
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
