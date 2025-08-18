use crate::kopia::{Snapshot, get_retention_counts};
use std::fmt::{self, Display};

struct MetricLabel {
    pub name: &'static str,
    pub help_text: &'static str,
    pub ty: MetricType,
}
enum MetricType {
    Gauge,
}
impl MetricLabel {
    pub const fn gauge(name: &'static str, help_text: &'static str) -> Self {
        Self {
            name,
            help_text,
            ty: MetricType::Gauge,
        }
    }
}
impl Display for MetricLabel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Self {
            name,
            help_text,
            ty,
        } = self;
        let ty = match ty {
            MetricType::Gauge => "gauge",
        };

        write!(f, "# HELP {name} {help_text}")?;
        writeln!(f)?;
        write!(f, "# TYPE {name} {ty}")?;

        Ok(())
    }
}

/// Generates Prometheus metrics for snapshots by retention reason.
///
/// Returns a string containing Prometheus-formatted metrics showing the count
/// of snapshots for each retention reason (e.g., "latest-1", "daily-7", etc.).
#[must_use]
pub fn snapshots_by_retention_metrics(snapshots: &[Snapshot]) -> impl Display {
    const NAME: &str = "kopia_snapshots_by_retention";
    const LABEL: MetricLabel = MetricLabel::gauge(NAME, "Number of snapshots by retention reason");

    struct Output {
        retention_counts: std::collections::BTreeMap<String, u32>,
    }
    impl Display for Output {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            let Self { retention_counts } = self;
            writeln!(f, "{LABEL}")?;
            for (reason, count) in retention_counts {
                writeln!(f, "{NAME}{{retention_reason=\"{reason}\"}} {count}")?;
            }
            Ok(())
        }
    }

    let retention_counts = get_retention_counts(snapshots);
    Output { retention_counts }
}

/// Generates Prometheus metrics for the latest snapshot size.
///
/// Returns a string containing Prometheus-formatted metrics showing the total
/// size in bytes of the most recent snapshot.
#[must_use]
pub fn latest_snapshot_size_metrics(snapshots: &[Snapshot]) -> impl Display {
    const NAME: &str = "kopia_snapshot_total_size_bytes";
    const LABEL: MetricLabel = MetricLabel::gauge(NAME, "Total size of latest snapshot in bytes");

    struct Output {
        total_size: u64,
    }
    impl Display for Output {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            let Self { total_size } = self;

            writeln!(f, "{LABEL}")?;
            writeln!(f, "{NAME} {total_size}")
        }
    }

    let total_size = snapshots.last().map_or(0, |v| v.stats.total_size);
    Output { total_size }
}

/// Generates Prometheus metrics for the age of the latest snapshot.
///
/// Returns a string containing Prometheus-formatted metrics showing the age
/// in seconds of the most recent snapshot from its end time.
#[must_use]
fn snapshot_age_metrics(snapshots: &[Snapshot], now: jiff::Timestamp) -> impl Display {
    const NAME: &str = "kopia_snapshot_age_seconds";
    const LABEL: MetricLabel = MetricLabel::gauge(NAME, "Age of newest snapshot in seconds");

    struct Output {
        age_seconds: i64,
    }
    impl Display for Output {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            let Self { age_seconds } = self;
            writeln!(f, "{LABEL}")?;
            writeln!(f, "{NAME} {age_seconds}")
        }
    }

    let age_seconds = snapshots
        .last()
        .and_then(|latest| {
            let end_time: jiff::Timestamp = latest.end_time.parse().ok()?;
            let age = now - end_time;
            let age_seconds = age
                .total(jiff::Unit::Second)
                .expect("relative reference time given");
            #[allow(clippy::cast_possible_truncation)]
            Some(age_seconds.round() as i64)
        })
        .unwrap_or(0);
    Output { age_seconds }
}

/// Generates Prometheus metrics for errors in the latest snapshot.
///
/// Returns a string containing Prometheus-formatted metrics showing the total
/// number of errors in the most recent snapshot.
#[must_use]
pub fn snapshot_errors_metrics(snapshots: &[Snapshot]) -> impl Display {
    const NAME: &str = "kopia_snapshot_errors_total";
    const LABEL: MetricLabel = MetricLabel::gauge(NAME, "Total errors in latest snapshot");

    struct Output {
        error_count: u32,
    }
    impl Display for Output {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            let Self { error_count } = self;
            writeln!(f, "{LABEL}")?;
            writeln!(f, "{NAME} {error_count}")
        }
    }

    let error_count = snapshots.last().map_or(0, |v| v.stats.error_count);
    Output { error_count }
}

/// Generates Prometheus metrics for failed files in the latest snapshot.
///
/// Returns a string containing Prometheus-formatted metrics showing the number
/// of failed files in the most recent snapshot.
#[must_use]
pub fn snapshot_failed_files_metrics(snapshots: &[Snapshot]) -> impl Display {
    const NAME: &str = "kopia_snapshot_failed_files_total";
    const LABEL: MetricLabel =
        MetricLabel::gauge(NAME, "Number of failed files in latest snapshot");

    struct Output {
        num_failed: u32,
    }
    impl Display for Output {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            let Self { num_failed } = self;
            writeln!(f, "{LABEL}")?;
            writeln!(f, "{NAME} {num_failed}")
        }
    }

    let num_failed = snapshots.last().map_or(0, |v| v.root_entry.summ.num_failed);
    Output { num_failed }
}

/// Generates Prometheus metrics for the total number of snapshots.
///
/// Returns a string containing Prometheus-formatted metrics showing the total
/// count of all snapshots in the repository.
#[must_use]
pub fn snapshots_total_metrics(snapshots: &[Snapshot]) -> impl Display {
    const NAME: &str = "kopia_snapshots_total";
    const LABEL: MetricLabel = MetricLabel::gauge(NAME, "Total number of snapshots");

    struct Output {
        count: usize,
    }
    impl Display for Output {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            let Self { count } = self;
            writeln!(f, "{LABEL}")?;
            writeln!(f, "{NAME} {count}")
        }
    }

    let count = snapshots.len();
    Output { count }
}

/// Generates all Prometheus metrics for the `/metrics` endpoint.
///
/// Combines all available metrics into a single response suitable for
/// Prometheus scraping.
#[must_use]
pub fn generate_all_metrics(snapshots: &[Snapshot], now: jiff::Timestamp) -> String {
    let metrics: &[&dyn Display] = &[
        &snapshots_by_retention_metrics(snapshots),
        &latest_snapshot_size_metrics(snapshots),
        &snapshot_age_metrics(snapshots, now),
        &snapshot_errors_metrics(snapshots),
        &snapshot_failed_files_metrics(snapshots),
        &snapshots_total_metrics(snapshots),
    ];

    let mut output = String::new();

    let mut first = Some(());
    for metric in metrics {
        use std::fmt::Write as _;

        if first.take().is_none() {
            output.push('\n');
        }
        write!(&mut output, "{metric}").expect("infallible");
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::kopia::{RootEntry, Snapshot, Source, Stats, Summary};

    fn create_test_snapshot(id: &str, total_size: u64, retention_reasons: &[&str]) -> Snapshot {
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
            retention_reason: retention_reasons.iter().map(ToString::to_string).collect(),
        }
    }

    #[test]
    fn test_snapshots_by_retention_metrics() {
        let snapshots = &[
            create_test_snapshot("1", 1000, &["latest-1", "daily-1"]),
            create_test_snapshot("2", 2000, &["daily-2"]),
        ];

        let metrics = snapshots_by_retention_metrics(snapshots).to_string();

        assert!(metrics.contains("# HELP kopia_snapshots_by_retention"));
        assert!(metrics.contains("# TYPE kopia_snapshots_by_retention gauge"));
        assert!(metrics.contains("kopia_snapshots_by_retention{retention_reason=\"latest-1\"} 1"));
        assert!(metrics.contains("kopia_snapshots_by_retention{retention_reason=\"daily-1\"} 1"));
        assert!(metrics.contains("kopia_snapshots_by_retention{retention_reason=\"daily-2\"} 1"));
    }

    #[test]
    fn test_latest_snapshot_size_metrics() {
        let snapshots = vec![
            create_test_snapshot("1", 1000, &["daily-2"]),
            create_test_snapshot("2", 2000, &["latest-1"]),
        ];

        let metrics = latest_snapshot_size_metrics(&snapshots).to_string();

        assert!(metrics.contains("# HELP kopia_snapshot_total_size_bytes"));
        assert!(metrics.contains("# TYPE kopia_snapshot_total_size_bytes gauge"));
        assert!(metrics.contains("kopia_snapshot_total_size_bytes 2000"));
    }

    #[test]
    fn test_latest_snapshot_size_metrics_empty() {
        let snapshots = vec![];
        let metrics = latest_snapshot_size_metrics(&snapshots).to_string();

        assert!(metrics.contains("kopia_snapshot_total_size_bytes 0"));
    }

    #[test]
    fn test_snapshot_age_metrics() {
        use jiff::ToSpan as _;
        let now = jiff::Timestamp::now();
        let recent_time = now - 30.minutes();
        let mut snapshot = create_test_snapshot("1", 1000, &["latest-1"]);
        snapshot.end_time = recent_time.to_string();

        let metrics = snapshot_age_metrics(&[snapshot], now).to_string();

        assert!(metrics.contains("# HELP kopia_snapshot_age_seconds"));
        assert!(metrics.contains("# TYPE kopia_snapshot_age_seconds gauge"));

        let age_line = metrics
            .lines()
            .find(|line| line.starts_with("kopia_snapshot_age_seconds "))
            .expect("Should contain age metric");

        let age_value: i64 = age_line
            .split_whitespace()
            .nth(1)
            .expect("Should have age value")
            .parse()
            .expect("Age should be a valid number");

        assert!(age_value >= 1770); // At least 29.5 minutes
        assert!(age_value <= 1890); // At most 31.5 minutes
    }

    #[test]
    fn test_snapshot_age_metrics_empty() {
        let snapshots = vec![];
        let now = jiff::Timestamp::now();
        let metrics = snapshot_age_metrics(&snapshots, now).to_string();

        assert!(metrics.contains("kopia_snapshot_age_seconds 0"));
    }

    #[test]
    fn test_snapshot_age_metrics_invalid_time() {
        let mut snapshot = create_test_snapshot("1", 1000, &["latest-1"]);
        snapshot.end_time = "invalid-time".to_string();

        let now = jiff::Timestamp::now();

        let metrics = snapshot_age_metrics(&[snapshot], now).to_string();

        assert!(metrics.contains("kopia_snapshot_age_seconds 0"));
    }

    #[test]
    fn test_snapshot_errors_metrics() {
        let mut snapshot = create_test_snapshot("1", 1000, &["latest-1"]);
        snapshot.stats.error_count = 5;

        let metrics = snapshot_errors_metrics(&[snapshot]).to_string();

        assert!(metrics.contains("# HELP kopia_snapshot_errors_total"));
        assert!(metrics.contains("# TYPE kopia_snapshot_errors_total gauge"));
        assert!(metrics.contains("kopia_snapshot_errors_total 5"));
    }

    #[test]
    fn test_snapshot_errors_metrics_no_errors() {
        let snapshot = create_test_snapshot("1", 1000, &["latest-1"]);

        let metrics = snapshot_errors_metrics(&[snapshot]).to_string();

        assert!(metrics.contains("kopia_snapshot_errors_total 0"));
    }

    #[test]
    fn test_snapshot_errors_metrics_empty() {
        let snapshots = vec![];
        let metrics = snapshot_errors_metrics(&snapshots).to_string();

        assert!(metrics.contains("kopia_snapshot_errors_total 0"));
    }

    #[test]
    fn test_snapshot_failed_files_metrics() {
        let mut snapshot = create_test_snapshot("1", 1000, &["latest-1"]);
        snapshot.root_entry.summ.num_failed = 3;

        let metrics = snapshot_failed_files_metrics(&[snapshot]).to_string();

        assert!(metrics.contains("# HELP kopia_snapshot_failed_files_total"));
        assert!(metrics.contains("# TYPE kopia_snapshot_failed_files_total gauge"));
        assert!(metrics.contains("kopia_snapshot_failed_files_total 3"));
    }

    #[test]
    fn test_snapshot_failed_files_metrics_no_failures() {
        let snapshot = create_test_snapshot("1", 1000, &["latest-1"]);

        let metrics = snapshot_failed_files_metrics(&[snapshot]).to_string();

        assert!(metrics.contains("kopia_snapshot_failed_files_total 0"));
    }

    #[test]
    fn test_snapshot_failed_files_metrics_empty() {
        let snapshots = vec![];
        let metrics = snapshot_failed_files_metrics(&snapshots).to_string();

        assert!(metrics.contains("kopia_snapshot_failed_files_total 0"));
    }

    #[test]
    fn test_snapshots_total_metrics() {
        let snapshots = vec![
            create_test_snapshot("1", 1000, &["latest-1"]),
            create_test_snapshot("2", 2000, &["daily-1"]),
            create_test_snapshot("3", 3000, &["monthly-1"]),
        ];

        let metrics = snapshots_total_metrics(&snapshots).to_string();

        assert!(metrics.contains("# HELP kopia_snapshots_total"));
        assert!(metrics.contains("# TYPE kopia_snapshots_total gauge"));
        assert!(metrics.contains("kopia_snapshots_total 3"));
    }

    #[test]
    fn test_snapshots_total_metrics_empty() {
        let snapshots = vec![];
        let metrics = snapshots_total_metrics(&snapshots).to_string();

        assert!(metrics.contains("kopia_snapshots_total 0"));
    }

    #[test]
    fn test_snapshots_total_metrics_single() {
        let snapshots = vec![create_test_snapshot("1", 1000, &["latest-1"])];
        let metrics = snapshots_total_metrics(&snapshots).to_string();

        assert!(metrics.contains("kopia_snapshots_total 1"));
    }

    #[test]
    fn test_generate_all_metrics() {
        let snapshots = vec![create_test_snapshot("1", 1000, &["daily-1"])];

        let now = jiff::Timestamp::now();

        let metrics = generate_all_metrics(&snapshots, now);

        assert!(metrics.contains("kopia_snapshots_by_retention"));
        assert!(metrics.contains("kopia_snapshot_total_size_bytes"));
        assert!(metrics.contains("kopia_snapshot_age_seconds"));
        assert!(metrics.contains("kopia_snapshot_errors_total"));
        assert!(metrics.contains("kopia_snapshot_failed_files_total"));
        assert!(metrics.contains("kopia_snapshots_total"));
    }

    #[test]
    fn snapshot() {
        let sample_data = include_str!("sample_kopia-snapshot-list.json");
        let snapshots = crate::parse_snapshots(sample_data).expect("valid snapshot JSON");

        let now: jiff::Timestamp = "2025-08-17T20:58:04.972143344Z"
            .parse()
            .expect("valid timestamp");

        insta::assert_snapshot!(
            generate_all_metrics(&snapshots, now),
            @r#"
            # HELP kopia_snapshots_by_retention Number of snapshots by retention reason
            # TYPE kopia_snapshots_by_retention gauge
            kopia_snapshots_by_retention{retention_reason="annual-1"} 1
            kopia_snapshots_by_retention{retention_reason="daily-1"} 1
            kopia_snapshots_by_retention{retention_reason="daily-2"} 1
            kopia_snapshots_by_retention{retention_reason="daily-3"} 1
            kopia_snapshots_by_retention{retention_reason="daily-4"} 1
            kopia_snapshots_by_retention{retention_reason="daily-5"} 1
            kopia_snapshots_by_retention{retention_reason="daily-6"} 1
            kopia_snapshots_by_retention{retention_reason="hourly-1"} 1
            kopia_snapshots_by_retention{retention_reason="hourly-2"} 1
            kopia_snapshots_by_retention{retention_reason="hourly-3"} 1
            kopia_snapshots_by_retention{retention_reason="hourly-4"} 1
            kopia_snapshots_by_retention{retention_reason="hourly-5"} 1
            kopia_snapshots_by_retention{retention_reason="latest-1"} 1
            kopia_snapshots_by_retention{retention_reason="latest-10"} 1
            kopia_snapshots_by_retention{retention_reason="latest-2"} 1
            kopia_snapshots_by_retention{retention_reason="latest-3"} 1
            kopia_snapshots_by_retention{retention_reason="latest-4"} 1
            kopia_snapshots_by_retention{retention_reason="latest-5"} 1
            kopia_snapshots_by_retention{retention_reason="latest-6"} 1
            kopia_snapshots_by_retention{retention_reason="latest-7"} 1
            kopia_snapshots_by_retention{retention_reason="latest-8"} 1
            kopia_snapshots_by_retention{retention_reason="latest-9"} 1
            kopia_snapshots_by_retention{retention_reason="monthly-1"} 1
            kopia_snapshots_by_retention{retention_reason="monthly-2"} 1
            kopia_snapshots_by_retention{retention_reason="monthly-3"} 1
            kopia_snapshots_by_retention{retention_reason="monthly-4"} 1
            kopia_snapshots_by_retention{retention_reason="weekly-1"} 1
            kopia_snapshots_by_retention{retention_reason="weekly-2"} 1
            kopia_snapshots_by_retention{retention_reason="weekly-3"} 1
            kopia_snapshots_by_retention{retention_reason="weekly-4"} 1

            # HELP kopia_snapshot_total_size_bytes Total size of latest snapshot in bytes
            # TYPE kopia_snapshot_total_size_bytes gauge
            kopia_snapshot_total_size_bytes 42154950324

            # HELP kopia_snapshot_age_seconds Age of newest snapshot in seconds
            # TYPE kopia_snapshot_age_seconds gauge
            kopia_snapshot_age_seconds 334678

            # HELP kopia_snapshot_errors_total Total errors in latest snapshot
            # TYPE kopia_snapshot_errors_total gauge
            kopia_snapshot_errors_total 0

            # HELP kopia_snapshot_failed_files_total Number of failed files in latest snapshot
            # TYPE kopia_snapshot_failed_files_total gauge
            kopia_snapshot_failed_files_total 0

            # HELP kopia_snapshots_total Total number of snapshots
            # TYPE kopia_snapshots_total gauge
            kopia_snapshots_total 17
            "#
        );
    }
}
