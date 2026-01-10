use crate::{
    SourceMap,
    kopia::{Snapshot, get_retention_counts},
};
use std::{
    collections::BTreeMap,
    fmt::{self, Display},
};

use self::last_snapshots::MetricLastSnapshots;

struct MetricLabel {
    name: &'static str,
    help_text: &'static str,
    ty: MetricType,
}
enum MetricType {
    Gauge,
}
impl MetricLabel {
    const fn gauge(name: &'static str, help_text: &'static str) -> Self {
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
fn snapshots_by_retention(snapshots_map: &SourceMap<Vec<Snapshot>>) -> impl Display {
    const NAME: &str = "kopia_snapshots_by_retention";
    const LABEL: MetricLabel = MetricLabel::gauge(NAME, "Number of snapshots by retention reason");

    struct Output {
        retention_counts: SourceMap<BTreeMap<String, u32>>,
    }
    impl Display for Output {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            let Self { retention_counts } = self;
            writeln!(f, "{LABEL}")?;
            for (source, reason_counts) in retention_counts {
                for (reason, count) in reason_counts {
                    writeln!(
                        f,
                        "{NAME}{{source={source:?},retention_reason={reason:?}}} {count}"
                    )?;
                }
            }
            Ok(())
        }
    }

    let retention_counts = get_retention_counts(snapshots_map);
    Output { retention_counts }
}

mod last_snapshots {
    use crate::{Snapshot, SourceMap, SourceStr, metrics::MetricLabel};
    use std::fmt::{self, Display};

    #[derive(Clone, Copy)]
    struct LastSnapshots<'a> {
        map: &'a SourceMap<Vec<Snapshot>>,
    }
    impl<'a> LastSnapshots<'a> {
        pub fn new(map: &'a SourceMap<Vec<Snapshot>>) -> Option<Self> {
            map.iter()
                .any(|(_source, snapshots)| !snapshots.is_empty())
                .then_some(Self { map })
        }
        pub fn iter(self) -> impl Iterator<Item = (&'a SourceStr, &'a Snapshot)> {
            let Self { map } = self;
            map.iter()
                .filter_map(|(source, snapshots)| snapshots.last().map(|last| (source, last)))
        }
    }

    pub struct MetricLastSnapshots<'a, F> {
        last_snapshots: LastSnapshots<'a>,
        name: &'static str,
        label: MetricLabel,
        stat_fn: F,
    }
    impl<'a, F, T> MetricLastSnapshots<'a, F>
    where
        F: Fn(&Snapshot) -> T,
        T: Display,
    {
        pub fn new(
            snapshots_map: &'a SourceMap<Vec<Snapshot>>,
            name: &'static str,
            label: MetricLabel,
            stat_fn: F,
        ) -> Option<Self> {
            let last_snapshots = LastSnapshots::new(snapshots_map)?;
            Some(Self {
                last_snapshots,
                name,
                label,
                stat_fn,
            })
        }
    }
    impl<F, T> Display for MetricLastSnapshots<'_, F>
    where
        F: Fn(&Snapshot) -> T,
        T: Display,
    {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            let Self {
                last_snapshots,
                name,
                label,
                stat_fn,
            } = self;
            writeln!(f, "{label}")?;
            for (source, last) in last_snapshots.iter() {
                let stat = stat_fn(last);
                writeln!(f, "{name}{{source={source:?}}} {stat}")?;
            }
            Ok(())
        }
    }
}

/// Generates Prometheus metrics for the latest snapshot size.
///
/// Returns a string containing Prometheus-formatted metrics showing the total
/// size in bytes of the most recent snapshot. Only present if snapshots list is not empty.
#[must_use]
fn snapshot_total_size_bytes(snapshots_map: &SourceMap<Vec<Snapshot>>) -> Option<impl Display> {
    const NAME: &str = "kopia_snapshot_total_size_bytes";
    const LABEL: MetricLabel = MetricLabel::gauge(NAME, "Total size of latest snapshot in bytes");

    MetricLastSnapshots::new(snapshots_map, NAME, LABEL, |v| v.stats.total_size)
}

/// Generates Prometheus metrics for the age of the latest snapshot.
///
/// Returns a string containing Prometheus-formatted metrics showing the age
/// in seconds of the most recent snapshot from its end time. Only present if snapshots list is not empty.
#[must_use]
fn snapshot_age_seconds(
    snapshots_map: &SourceMap<Vec<Snapshot>>,
    now: jiff::Timestamp,
) -> Option<impl Display> {
    const NAME: &str = "kopia_snapshot_age_seconds";
    const LABEL: MetricLabel = MetricLabel::gauge(NAME, "Age of newest snapshot in seconds");

    struct Output {
        age_seconds_map: SourceMap<i64>,
    }
    impl Display for Output {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            let Self { age_seconds_map } = self;
            writeln!(f, "{LABEL}")?;
            for (source, age_seconds) in age_seconds_map {
                writeln!(f, "{NAME}{{source={source:?}}} {age_seconds}")?;
            }

            Ok(())
        }
    }

    let age_seconds_map: SourceMap<_> = snapshots_map
        .iter()
        .filter_map(|(source, snapshots)| {
            let last = snapshots.last()?;
            let age_seconds = {
                let end_time: jiff::Timestamp = last.end_time.parse().ok()?;
                let age = now - end_time;
                let age_seconds = age
                    .total(jiff::Unit::Second)
                    .expect("relative reference time given");
                #[allow(clippy::cast_possible_truncation)]
                {
                    age_seconds.round() as i64
                }
            };
            Some((source.clone(), age_seconds))
        })
        .collect();
    if age_seconds_map.is_empty() {
        None
    } else {
        Some(Output { age_seconds_map })
    }
}

/// Generates Prometheus metrics for timestamp parsing errors.
///
/// Returns a string containing Prometheus-formatted metrics showing the count
/// of snapshots with unparseable timestamps. Only present if there are parsing errors.
#[must_use]
fn snapshot_timestamp_parse_errors_total(
    snapshots_map: &SourceMap<Vec<Snapshot>>,
) -> Option<impl Display> {
    const NAME: &str = "kopia_snapshot_timestamp_parse_errors_total";
    const LABEL: MetricLabel =
        MetricLabel::gauge(NAME, "Number of snapshots with unparseable timestamps");

    struct ErrorCounts(SourceMap<u32>);
    impl Display for ErrorCounts {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            let Self(error_counts) = self;
            writeln!(f, "{LABEL}")?;
            for (source, error_count) in error_counts {
                writeln!(f, "{NAME}{{source={source:?}}} {error_count}")?;
            }
            Ok(())
        }
    }

    let error_counts: SourceMap<u32> = snapshots_map
        .iter()
        .filter_map(|(source, snapshots)| {
            let error_count = snapshots
                .iter()
                .map(|snapshot| {
                    if snapshot.end_time.parse::<jiff::Timestamp>().is_err() {
                        1
                    } else {
                        0
                    }
                })
                .sum::<u32>();

            (error_count > 0).then(|| (source.clone(), error_count))
        })
        .collect();

    error_counts.map_nonempty(ErrorCounts)
}

/// Generates Prometheus metrics for the last successful snapshot timestamp.
///
/// Returns a string containing Prometheus-formatted metrics showing the Unix timestamp
/// of the most recent snapshot. Only present if snapshots list is not empty.
#[must_use]
fn snapshot_last_success_timestamp(
    snapshots_map: &SourceMap<Vec<Snapshot>>,
) -> Option<impl Display> {
    const NAME: &str = "kopia_snapshot_last_success_timestamp";
    const LABEL: MetricLabel =
        MetricLabel::gauge(NAME, "Unix timestamp of last successful snapshot");

    struct Timestamps(SourceMap<i64>);
    impl Display for Timestamps {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            let Self(timestamps) = self;
            writeln!(f, "{LABEL}")?;
            for (source, timestamp) in timestamps {
                writeln!(f, "{NAME}{{source={source:?}}} {timestamp}")?;
            }
            Ok(())
        }
    }

    let timestamps: SourceMap<i64> = snapshots_map
        .iter()
        .filter_map(|(source, snapshots)| {
            let last = snapshots.last()?;
            let end_time: jiff::Timestamp = last.end_time.parse().ok()?;
            Some((source.clone(), end_time.as_second()))
        })
        .collect();

    timestamps.map_nonempty(Timestamps)
}

/// Generates Prometheus metrics for errors in the latest snapshot.
///
/// Returns a string containing Prometheus-formatted metrics showing the total
/// number of errors in the most recent snapshot. Only present if snapshots list is not empty.
#[must_use]
fn snapshot_errors_total(snapshots_map: &SourceMap<Vec<Snapshot>>) -> Option<impl Display> {
    const NAME: &str = "kopia_snapshot_errors_total";
    const LABEL: MetricLabel = MetricLabel::gauge(NAME, "Total errors in latest snapshot");

    MetricLastSnapshots::new(snapshots_map, NAME, LABEL, |v| v.stats.error_count)
}

/// Generates Prometheus metrics for ignored errors in the latest snapshot.
///
/// Returns a string containing Prometheus-formatted metrics showing the total
/// number of ignored errors in the most recent snapshot. Only present if snapshots list is not empty.
#[must_use]
fn snapshot_ignored_errors_total(snapshots_map: &SourceMap<Vec<Snapshot>>) -> Option<impl Display> {
    const NAME: &str = "kopia_snapshot_ignored_errors_total";
    const LABEL: MetricLabel = MetricLabel::gauge(NAME, "Ignored errors in latest snapshot");

    MetricLastSnapshots::new(snapshots_map, NAME, LABEL, |v| v.stats.ignored_error_count)
}

/// Generates Prometheus metrics for failed files in the latest snapshot.
///
/// Returns a string containing Prometheus-formatted metrics showing the number
/// of failed files in the most recent snapshot. Only present if snapshots list is not empty.
#[must_use]
fn snapshot_failed_files_total(snapshots_map: &SourceMap<Vec<Snapshot>>) -> Option<impl Display> {
    const NAME: &str = "kopia_snapshot_failed_files_total";
    const LABEL: MetricLabel =
        MetricLabel::gauge(NAME, "Number of failed files in latest snapshot");

    MetricLastSnapshots::new(snapshots_map, NAME, LABEL, |v| v.root_entry.summ.num_failed)
}

/// Generates Prometheus metrics for the size change from the previous snapshot.
///
/// Returns a string containing Prometheus-formatted metrics showing the change
/// in bytes from the previous snapshot. Only present if snapshots list has more than one snapshot.
#[must_use]
fn snapshot_size_change_bytes(snapshots_map: &SourceMap<Vec<Snapshot>>) -> Option<impl Display> {
    const NAME: &str = "kopia_snapshot_size_change_bytes";
    const LABEL: MetricLabel = MetricLabel::gauge(NAME, "Change in size from previous snapshot");

    struct SizeChanges(SourceMap<i128>);
    impl Display for SizeChanges {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            let Self(size_changes) = self;
            writeln!(f, "{LABEL}")?;
            for (source, size_change) in size_changes {
                writeln!(f, "{NAME}{{source={source:?}}} {size_change}")?;
            }
            Ok(())
        }
    }

    let size_changes: SourceMap<i128> = snapshots_map
        .iter()
        .filter_map(|(source, snapshots)| {
            let mut iter = snapshots.iter().rev();
            let latest = iter.next()?;
            let previous = iter.next()?;

            let latest_size: u64 = latest.stats.total_size;
            let previous_size: u64 = previous.stats.total_size;

            let size_change = u128::from(latest_size)
                .checked_signed_diff(u128::from(previous_size))
                .expect("u64 diff fits in i128");
            Some((source.clone(), size_change))
        })
        .collect();
    size_changes.map_nonempty(SizeChanges)
}

/// Generates Prometheus metrics for the total number of snapshots.
///
/// Returns a string containing Prometheus-formatted metrics showing the total
/// count of all snapshots in the repository.
#[must_use]
fn snapshots_total(snapshots_map: &SourceMap<Vec<Snapshot>>) -> impl Display {
    const NAME: &str = "kopia_snapshots_total";
    const LABEL: MetricLabel = MetricLabel::gauge(NAME, "Total number of snapshots");

    struct Output<'a> {
        snapshots_map: &'a SourceMap<Vec<Snapshot>>,
    }
    impl Display for Output<'_> {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            let Self { snapshots_map } = *self;
            writeln!(f, "{LABEL}")?;
            for (source, snapshots) in snapshots_map {
                let count = snapshots.len();
                writeln!(f, "{NAME}{{source={source:?}}} {count}")?;
            }
            Ok(())
        }
    }

    Output { snapshots_map }
}

/// Generates all Prometheus metrics for the `/metrics` endpoint.
///
/// Combines all available metrics into a single response suitable for
/// Prometheus scraping.
#[must_use]
pub fn generate_all_metrics(
    snapshots_map: &SourceMap<Vec<Snapshot>>,
    now: jiff::Timestamp,
) -> String {
    struct Accumulator {
        output: String,
        first: Option<()>,
    }
    impl Accumulator {
        fn new() -> Self {
            Self {
                output: String::new(),
                first: Some(()),
            }
        }
        fn push(mut self, metric: Option<impl Display>) -> Self {
            use std::fmt::Write as _;
            if let Some(m) = metric {
                let Self { first, output } = &mut self;
                if first.take().is_none() {
                    output.push('\n');
                }
                write!(output, "{m}").expect("infallible");
            }
            self
        }
        fn finish(self) -> String {
            self.output
        }
    }

    Accumulator::new()
        .push(Some(snapshots_by_retention(snapshots_map)))
        .push(snapshot_total_size_bytes(snapshots_map))
        .push(snapshot_age_seconds(snapshots_map, now))
        .push(snapshot_timestamp_parse_errors_total(snapshots_map))
        .push(snapshot_last_success_timestamp(snapshots_map))
        .push(snapshot_errors_total(snapshots_map))
        .push(snapshot_ignored_errors_total(snapshots_map))
        .push(snapshot_failed_files_total(snapshots_map))
        .push(snapshot_size_change_bytes(snapshots_map))
        .push(Some(snapshots_total(snapshots_map)))
        .finish()
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)] // tests can unwrap
    #![allow(clippy::panic)] // tests can panic

    use super::*;
    use crate::{
        kopia::{RootEntry, Snapshot, Source, Stats, Summary},
        test_util::single_map,
    };

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
        let (map, _source) = &single_map(vec![
            create_test_snapshot("1", 1000, &["latest-1", "daily-1"]),
            create_test_snapshot("2", 2000, &["daily-2"]),
        ]);

        let metrics = snapshots_by_retention(map).to_string();

        assert!(metrics.contains("# HELP kopia_snapshots_by_retention"));
        assert!(metrics.contains("# TYPE kopia_snapshots_by_retention gauge"));
        assert!(metrics.contains(r#"kopia_snapshots_by_retention{source="user_name@host:/path",retention_reason="latest-1"} 1"#));
        assert!(metrics.contains(r#"kopia_snapshots_by_retention{source="user_name@host:/path",retention_reason="daily-1"} 1"#));
        assert!(metrics.contains(r#"kopia_snapshots_by_retention{source="user_name@host:/path",retention_reason="daily-2"} 1"#));
    }

    #[test]
    fn test_latest_snapshot_size_metrics() {
        let (map, _source) = single_map(vec![
            create_test_snapshot("1", 1000, &["daily-2"]),
            create_test_snapshot("2", 2000, &["latest-1"]),
        ]);

        let metrics = snapshot_total_size_bytes(&map)
            .expect("nonempty")
            .to_string();

        assert!(metrics.contains("# HELP kopia_snapshot_total_size_bytes"));
        assert!(metrics.contains("# TYPE kopia_snapshot_total_size_bytes gauge"));
        assert!(
            metrics
                .contains(r#"kopia_snapshot_total_size_bytes{source="user_name@host:/path"} 2000"#),
            "{metrics:?}"
        );
    }

    #[test]
    fn test_latest_snapshot_size_metrics_empty() {
        let (map, _source) = single_map(vec![]);
        let metrics = snapshot_total_size_bytes(&map);

        assert!(metrics.is_none());
    }

    #[test]
    fn test_snapshot_age_metrics() {
        use jiff::ToSpan as _;

        for minutes in [30, 100] {
            let now = jiff::Timestamp::now();
            let recent_time = now - minutes.minutes();
            let mut snapshot = create_test_snapshot("1", 1000, &["latest-1"]);
            snapshot.end_time = recent_time.to_string();

            let (map, _source) = single_map(vec![snapshot]);

            let metrics = snapshot_age_seconds(&map, now)
                .expect("nonempty")
                .to_string();

            assert!(metrics.contains("# HELP kopia_snapshot_age_seconds"));
            assert!(metrics.contains("# TYPE kopia_snapshot_age_seconds gauge"));

            let Some(age_line) = metrics.lines().find(|line| {
                line.starts_with(r#"kopia_snapshot_age_seconds{source="user_name@host:/path"} "#)
            }) else {
                panic!("Should contain age metric: {metrics:?}")
            };

            let age_value: i64 = age_line
                .split_whitespace()
                .nth(1)
                .expect("Should have age value")
                .parse()
                .expect("Age should be a valid number");

            assert_eq!(age_value, minutes * 60); // exactly X minutes
        }
    }

    #[test]
    fn test_snapshot_age_metrics_empty() {
        let (map, _source) = single_map(vec![]);
        let now = jiff::Timestamp::now();
        let metrics = snapshot_age_seconds(&map, now);

        assert!(metrics.is_none());
    }

    #[test]
    fn test_snapshot_age_metric_invalid_time() {
        let mut snapshot = create_test_snapshot("1", 1000, &["latest-1"]);
        snapshot.end_time = "invalid-time".to_string();

        let now = jiff::Timestamp::now();

        let (map, _source) = single_map(vec![snapshot]);
        let age_metrics = snapshot_age_seconds(&map, now);
        let time_error_metrics = snapshot_timestamp_parse_errors_total(&map)
            .expect("nonempty")
            .to_string();

        assert!(age_metrics.is_none());
        assert!(time_error_metrics.contains(
            r#"kopia_snapshot_timestamp_parse_errors_total{source="user_name@host:/path"} 1"#
        ));
    }

    #[test]
    fn test_snapshot_errors_metrics() {
        let mut snapshot = create_test_snapshot("1", 1000, &["latest-1"]);
        snapshot.stats.error_count = 5;

        let (map, _source) = single_map(vec![snapshot]);
        let metrics = snapshot_errors_total(&map).expect("nonempty").to_string();

        assert!(metrics.contains("# HELP kopia_snapshot_errors_total"));
        assert!(metrics.contains("# TYPE kopia_snapshot_errors_total gauge"));
        assert!(
            metrics.contains(r#"kopia_snapshot_errors_total{source="user_name@host:/path"} 5"#)
        );
    }

    #[test]
    fn test_snapshot_errors_metrics_no_errors() {
        let snapshot = create_test_snapshot("1", 1000, &["latest-1"]);

        let (map, _source) = single_map(vec![snapshot]);
        let metrics = snapshot_errors_total(&map).expect("nonempty").to_string();

        assert!(
            metrics.contains(r#"kopia_snapshot_errors_total{source="user_name@host:/path"} 0"#)
        );
    }

    #[test]
    fn test_snapshot_errors_metrics_empty() {
        let snapshots = vec![];
        let (map, _source) = single_map(snapshots);
        let metrics = snapshot_errors_total(&map);

        assert!(metrics.is_none());
    }

    #[test]
    fn test_snapshot_failed_files_metrics() {
        let mut snapshot = create_test_snapshot("1", 1000, &["latest-1"]);
        snapshot.root_entry.summ.num_failed = 3;

        let (map, _source) = single_map(vec![snapshot]);
        let metrics = snapshot_failed_files_total(&map)
            .expect("nonempty")
            .to_string();

        assert!(metrics.contains("# HELP kopia_snapshot_failed_files_total"));
        assert!(metrics.contains("# TYPE kopia_snapshot_failed_files_total gauge"));
        assert!(
            metrics
                .contains(r#"kopia_snapshot_failed_files_total{source="user_name@host:/path"} 3"#)
        );
    }

    #[test]
    fn test_snapshot_failed_files_metrics_no_failures() {
        let snapshot = create_test_snapshot("1", 1000, &["latest-1"]);

        let (map, _source) = single_map(vec![snapshot]);
        let metrics = snapshot_failed_files_total(&map)
            .expect("nonempty")
            .to_string();

        assert!(
            metrics
                .contains(r#"kopia_snapshot_failed_files_total{source="user_name@host:/path"} 0"#)
        );
    }

    #[test]
    fn test_snapshot_failed_files_metrics_empty() {
        let snapshots = vec![];
        let (map, _source) = &single_map(snapshots);
        let metrics = snapshot_failed_files_total(map);

        assert!(metrics.is_none());
    }

    #[test]
    fn test_snapshots_total_metrics() {
        let snapshots = vec![
            create_test_snapshot("1", 1000, &["latest-1"]),
            create_test_snapshot("2", 2000, &["daily-1"]),
            create_test_snapshot("3", 3000, &["monthly-1"]),
        ];

        let (map, _source) = single_map(snapshots);
        let metrics = snapshots_total(&map).to_string();

        assert!(metrics.contains("# HELP kopia_snapshots_total"));
        assert!(metrics.contains("# TYPE kopia_snapshots_total gauge"));
        assert!(metrics.contains(r#"kopia_snapshots_total{source="user_name@host:/path"} 3"#));
    }

    #[test]
    fn test_snapshots_total_metrics_empty() {
        let snapshots = vec![];
        let (map, _source) = single_map(snapshots);
        let metrics = snapshots_total(&map).to_string();

        assert!(metrics.contains(r#"kopia_snapshots_total{source="user_name@host:/path"} 0"#));
    }

    #[test]
    fn test_snapshots_total_metrics_single() {
        let snapshots = vec![create_test_snapshot("1", 1000, &["latest-1"])];
        let (map, _source) = single_map(snapshots);
        let metrics = snapshots_total(&map).to_string();

        assert!(metrics.contains(r#"kopia_snapshots_total{source="user_name@host:/path"} 1"#));
    }

    #[test]
    fn test_generate_all_metrics() {
        let snapshots = vec![create_test_snapshot("1", 1000, &["daily-1"])];

        let now = jiff::Timestamp::now();

        let (map, _source) = single_map(snapshots);
        let metrics = generate_all_metrics(&map, now);

        assert!(metrics.contains("kopia_snapshots_by_retention"));
        assert!(metrics.contains("kopia_snapshot_total_size_bytes"));
        assert!(metrics.contains("kopia_snapshot_age_seconds"));
        assert!(metrics.contains("kopia_snapshot_errors_total"));
        assert!(metrics.contains("kopia_snapshot_failed_files_total"));
        assert!(metrics.contains("kopia_snapshots_total"));
    }

    #[test]
    fn full_snapshot() {
        let sample_data = include_str!("sample_kopia-snapshot-list.json");
        let snapshots =
            crate::parse_snapshots(sample_data, |e| eyre::bail!(e)).expect("valid snapshot JSON");

        let now: jiff::Timestamp = "2025-08-17T20:58:04.972143344Z"
            .parse()
            .expect("valid timestamp");

        insta::assert_snapshot!(
            generate_all_metrics(&snapshots, now),
            @r#"
            # HELP kopia_snapshots_by_retention Number of snapshots by retention reason
            # TYPE kopia_snapshots_by_retention gauge
            kopia_snapshots_by_retention{source="kopia-system@milton:/persist-home",retention_reason="annual-1"} 1
            kopia_snapshots_by_retention{source="kopia-system@milton:/persist-home",retention_reason="daily-1"} 1
            kopia_snapshots_by_retention{source="kopia-system@milton:/persist-home",retention_reason="daily-2"} 1
            kopia_snapshots_by_retention{source="kopia-system@milton:/persist-home",retention_reason="daily-3"} 1
            kopia_snapshots_by_retention{source="kopia-system@milton:/persist-home",retention_reason="daily-4"} 1
            kopia_snapshots_by_retention{source="kopia-system@milton:/persist-home",retention_reason="daily-5"} 1
            kopia_snapshots_by_retention{source="kopia-system@milton:/persist-home",retention_reason="daily-6"} 1
            kopia_snapshots_by_retention{source="kopia-system@milton:/persist-home",retention_reason="hourly-1"} 1
            kopia_snapshots_by_retention{source="kopia-system@milton:/persist-home",retention_reason="hourly-2"} 1
            kopia_snapshots_by_retention{source="kopia-system@milton:/persist-home",retention_reason="hourly-3"} 1
            kopia_snapshots_by_retention{source="kopia-system@milton:/persist-home",retention_reason="hourly-4"} 1
            kopia_snapshots_by_retention{source="kopia-system@milton:/persist-home",retention_reason="hourly-5"} 1
            kopia_snapshots_by_retention{source="kopia-system@milton:/persist-home",retention_reason="latest-1"} 1
            kopia_snapshots_by_retention{source="kopia-system@milton:/persist-home",retention_reason="latest-10"} 1
            kopia_snapshots_by_retention{source="kopia-system@milton:/persist-home",retention_reason="latest-2"} 1
            kopia_snapshots_by_retention{source="kopia-system@milton:/persist-home",retention_reason="latest-3"} 1
            kopia_snapshots_by_retention{source="kopia-system@milton:/persist-home",retention_reason="latest-4"} 1
            kopia_snapshots_by_retention{source="kopia-system@milton:/persist-home",retention_reason="latest-5"} 1
            kopia_snapshots_by_retention{source="kopia-system@milton:/persist-home",retention_reason="latest-6"} 1
            kopia_snapshots_by_retention{source="kopia-system@milton:/persist-home",retention_reason="latest-7"} 1
            kopia_snapshots_by_retention{source="kopia-system@milton:/persist-home",retention_reason="latest-8"} 1
            kopia_snapshots_by_retention{source="kopia-system@milton:/persist-home",retention_reason="latest-9"} 1
            kopia_snapshots_by_retention{source="kopia-system@milton:/persist-home",retention_reason="monthly-1"} 1
            kopia_snapshots_by_retention{source="kopia-system@milton:/persist-home",retention_reason="monthly-2"} 1
            kopia_snapshots_by_retention{source="kopia-system@milton:/persist-home",retention_reason="monthly-3"} 1
            kopia_snapshots_by_retention{source="kopia-system@milton:/persist-home",retention_reason="monthly-4"} 1
            kopia_snapshots_by_retention{source="kopia-system@milton:/persist-home",retention_reason="weekly-1"} 1
            kopia_snapshots_by_retention{source="kopia-system@milton:/persist-home",retention_reason="weekly-2"} 1
            kopia_snapshots_by_retention{source="kopia-system@milton:/persist-home",retention_reason="weekly-3"} 1
            kopia_snapshots_by_retention{source="kopia-system@milton:/persist-home",retention_reason="weekly-4"} 1

            # HELP kopia_snapshot_total_size_bytes Total size of latest snapshot in bytes
            # TYPE kopia_snapshot_total_size_bytes gauge
            kopia_snapshot_total_size_bytes{source="kopia-system@milton:/persist-home"} 42154950324

            # HELP kopia_snapshot_age_seconds Age of newest snapshot in seconds
            # TYPE kopia_snapshot_age_seconds gauge
            kopia_snapshot_age_seconds{source="kopia-system@milton:/persist-home"} 334678

            # HELP kopia_snapshot_last_success_timestamp Unix timestamp of last successful snapshot
            # TYPE kopia_snapshot_last_success_timestamp gauge
            kopia_snapshot_last_success_timestamp{source="kopia-system@milton:/persist-home"} 1755129606

            # HELP kopia_snapshot_errors_total Total errors in latest snapshot
            # TYPE kopia_snapshot_errors_total gauge
            kopia_snapshot_errors_total{source="kopia-system@milton:/persist-home"} 0

            # HELP kopia_snapshot_ignored_errors_total Ignored errors in latest snapshot
            # TYPE kopia_snapshot_ignored_errors_total gauge
            kopia_snapshot_ignored_errors_total{source="kopia-system@milton:/persist-home"} 0

            # HELP kopia_snapshot_failed_files_total Number of failed files in latest snapshot
            # TYPE kopia_snapshot_failed_files_total gauge
            kopia_snapshot_failed_files_total{source="kopia-system@milton:/persist-home"} 0

            # HELP kopia_snapshot_size_change_bytes Change in size from previous snapshot
            # TYPE kopia_snapshot_size_change_bytes gauge
            kopia_snapshot_size_change_bytes{source="kopia-system@milton:/persist-home"} 1116951

            # HELP kopia_snapshots_total Total number of snapshots
            # TYPE kopia_snapshots_total gauge
            kopia_snapshots_total{source="kopia-system@milton:/persist-home"} 17
            "#
        );
    }
}
