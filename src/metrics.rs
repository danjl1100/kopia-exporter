use crate::kopia::{Snapshot, get_retention_counts};

/// Generates Prometheus metrics for snapshots by retention reason.
///
/// Returns a string containing Prometheus-formatted metrics showing the count
/// of snapshots for each retention reason (e.g., "latest-1", "daily-7", etc.).
///
/// # Arguments
///
/// * `snapshots` - Slice of snapshots to analyze
///
/// # Examples
///
/// ```
/// # use kopia_exporter::metrics::snapshots_by_retention_metrics;
/// # use kopia_exporter::kopia::Snapshot;
/// let snapshots = vec![]; // Your snapshots here
/// let metrics = snapshots_by_retention_metrics(&snapshots);
/// println!("{}", metrics);
/// ```
#[must_use]
pub fn snapshots_by_retention_metrics(snapshots: &[Snapshot]) -> String {
    let mut output = String::new();

    output
        .push_str("# HELP kopia_snapshots_by_retention Number of snapshots by retention reason\n");
    output.push_str("# TYPE kopia_snapshots_by_retention gauge\n");

    let retention_counts = get_retention_counts(snapshots);

    for (reason, count) in &retention_counts {
        use std::fmt::Write;
        writeln!(
            output,
            "kopia_snapshots_by_retention{{retention_reason=\"{reason}\"}} {count}"
        )
        .expect("Writing to string should never fail");
    }

    output
}

/// Generates Prometheus metrics for the latest snapshot size.
///
/// Returns a string containing Prometheus-formatted metrics showing the total
/// size in bytes of the most recent snapshot.
///
/// # Arguments
///
/// * `snapshots` - Slice of snapshots to analyze (should be sorted by time)
///
/// # Examples
///
/// ```
/// # use kopia_exporter::metrics::latest_snapshot_size_metrics;
/// # use kopia_exporter::kopia::Snapshot;
/// let snapshots = vec![]; // Your snapshots here
/// let metrics = latest_snapshot_size_metrics(&snapshots);
/// println!("{}", metrics);
/// ```
#[must_use]
pub fn latest_snapshot_size_metrics(snapshots: &[Snapshot]) -> String {
    let mut output = String::new();

    output.push_str(
        "# HELP kopia_snapshot_total_size_bytes Total size of latest snapshot in bytes\n",
    );
    output.push_str("# TYPE kopia_snapshot_total_size_bytes gauge\n");

    if let Some(latest) = snapshots.last() {
        use std::fmt::Write;
        writeln!(
            output,
            "kopia_snapshot_total_size_bytes {}",
            latest.stats.total_size
        )
        .expect("Writing to string should never fail");
    } else {
        output.push_str("kopia_snapshot_total_size_bytes 0\n");
    }

    output
}

/// Generates all Prometheus metrics for the `/metrics` endpoint.
///
/// Combines all available metrics into a single response suitable for
/// Prometheus scraping.
///
/// # Arguments
///
/// * `snapshots` - Slice of snapshots to generate metrics from
///
/// # Examples
///
/// ```
/// # use kopia_exporter::metrics::generate_all_metrics;
/// # use kopia_exporter::kopia::Snapshot;
/// let snapshots = vec![]; // Your snapshots here
/// let metrics = generate_all_metrics(&snapshots);
/// println!("{}", metrics);
/// ```
#[must_use]
pub fn generate_all_metrics(snapshots: &[Snapshot]) -> String {
    let mut output = String::new();

    output.push_str(&snapshots_by_retention_metrics(snapshots));
    output.push('\n');
    output.push_str(&latest_snapshot_size_metrics(snapshots));

    output
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::kopia::{RootEntry, Snapshot, Source, Stats, Summary};

    fn create_test_snapshot(id: &str, total_size: u64, retention_reasons: Vec<&str>) -> Snapshot {
        Snapshot {
            id: id.to_string(),
            source: Source {
                host: "test".to_string(),
                user_name: "user".to_string(),
                path: "/test".to_string(),
            },
            description: "".to_string(),
            start_time: "2025-08-14T00:00:00Z".to_string(),
            end_time: "2025-08-14T00:01:00Z".to_string(),
            stats: Stats {
                total_size,
                excluded_total_size: 0,
                file_count: 10,
                cached_files: 5,
                non_cached_files: 5,
                dir_count: 2,
                excluded_file_count: 0,
                excluded_dir_count: 0,
                ignored_error_count: 0,
                error_count: 0,
            },
            root_entry: RootEntry {
                name: "test".to_string(),
                entry_type: "d".to_string(),
                mode: "0755".to_string(),
                mtime: "2025-08-14T00:00:00Z".to_string(),
                obj: "obj1".to_string(),
                summ: Summary {
                    size: total_size,
                    files: 10,
                    symlinks: 0,
                    dirs: 2,
                    max_time: "2025-08-14T00:00:00Z".to_string(),
                    num_failed: 0,
                },
            },
            retention_reason: retention_reasons.iter().map(|s| s.to_string()).collect(),
        }
    }

    #[test]
    fn test_snapshots_by_retention_metrics() {
        let snapshots = vec![
            create_test_snapshot("1", 1000, vec!["latest-1", "daily-1"]),
            create_test_snapshot("2", 2000, vec!["daily-2"]),
        ];

        let metrics = snapshots_by_retention_metrics(&snapshots);

        assert!(metrics.contains("# HELP kopia_snapshots_by_retention"));
        assert!(metrics.contains("# TYPE kopia_snapshots_by_retention gauge"));
        assert!(metrics.contains("kopia_snapshots_by_retention{retention_reason=\"latest-1\"} 1"));
        assert!(metrics.contains("kopia_snapshots_by_retention{retention_reason=\"daily-1\"} 1"));
        assert!(metrics.contains("kopia_snapshots_by_retention{retention_reason=\"daily-2\"} 1"));
    }

    #[test]
    fn test_latest_snapshot_size_metrics() {
        let snapshots = vec![
            create_test_snapshot("1", 1000, vec!["daily-2"]),
            create_test_snapshot("2", 2000, vec!["latest-1"]),
        ];

        let metrics = latest_snapshot_size_metrics(&snapshots);

        assert!(metrics.contains("# HELP kopia_snapshot_total_size_bytes"));
        assert!(metrics.contains("# TYPE kopia_snapshot_total_size_bytes gauge"));
        assert!(metrics.contains("kopia_snapshot_total_size_bytes 2000"));
    }

    #[test]
    fn test_latest_snapshot_size_metrics_empty() {
        let snapshots = vec![];
        let metrics = latest_snapshot_size_metrics(&snapshots);

        assert!(metrics.contains("kopia_snapshot_total_size_bytes 0"));
    }

    #[test]
    fn test_generate_all_metrics() {
        let snapshots = vec![create_test_snapshot("1", 1000, vec!["daily-1"])];

        let metrics = generate_all_metrics(&snapshots);

        assert!(metrics.contains("kopia_snapshots_by_retention"));
        assert!(metrics.contains("kopia_snapshot_total_size_bytes"));
    }
}
