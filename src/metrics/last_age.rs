use crate::{KopiaSnapshots, SourceMap, metrics::MetricLabel};
use std::fmt::{self, Display};

impl KopiaSnapshots {
    /// Generates Prometheus metrics for the age of the latest snapshot.
    ///
    /// Returns a string containing Prometheus-formatted metrics showing the age
    /// in seconds of the most recent snapshot from its end time. Only present if snapshots list is not empty.
    #[must_use]
    pub(super) fn snapshot_age_seconds(&self, now: jiff::Timestamp) -> Option<impl Display> {
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

        let age_seconds_map: SourceMap<_> = self
            .snapshots_map
            .iter()
            .filter_map(|(source, snapshots)| {
                let last = snapshots.last()?;
                let age_seconds = {
                    let age = now - last.end_time?;
                    let age_seconds = age
                        .total(jiff::Unit::Second)
                        .expect("relative reference time given");
                    #[expect(clippy::cast_possible_truncation)]
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
    pub(super) fn snapshot_timestamp_parse_errors_total(&self) -> Option<impl Display> {
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

        let error_counts: SourceMap<u32> = self
            .snapshots_map
            .iter()
            .filter_map(|(source, snapshots)| {
                let error_count = snapshots
                    .iter()
                    .map(|snapshot| if snapshot.end_time.is_none() { 1 } else { 0 })
                    .sum::<u32>();

                (error_count > 0).then(|| (source.clone(), error_count))
            })
            .collect();

        error_counts.map_nonempty(ErrorCounts)
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        AssertContains as _,
        test_util::{multi_map, single_map, test_snapshot},
    };

    #[test]
    fn snapshot_age_metrics() {
        use jiff::ToSpan as _;

        for minutes in [30, 100] {
            let now = jiff::Timestamp::now();
            let recent_time = now - minutes.minutes();
            let mut snapshot = test_snapshot("1", 1000, &["latest-1"]);
            snapshot.end_time = recent_time.to_string();

            let seconds = minutes * 60;

            let (map, _source) = single_map(vec![snapshot]);

            map.snapshot_age_seconds(now)
                .expect("nonempty")
                .assert_contains_snippets(&["# HELP kopia_snapshot_age_seconds"])
                .assert_contains_lines(&[
                    "# TYPE kopia_snapshot_age_seconds gauge",
                    &format!(
                        "kopia_snapshot_age_seconds{{source=\"user_name@host:/path\"}} {seconds}"
                    ),
                ]);
        }
    }

    #[test]
    fn snapshot_age_metrics_empty() {
        let (map, _source) = single_map(vec![]);
        let now = jiff::Timestamp::now();
        let metrics = map.snapshot_age_seconds(now);

        assert!(metrics.is_none());
    }

    #[test]
    fn snapshot_age_metric_invalid_time() {
        let mut snapshot = test_snapshot("1", 1000, &["latest-1"]);
        snapshot.end_time = "invalid-time".to_string();

        let now = jiff::Timestamp::now();

        let (map, _source) = single_map(vec![snapshot]);

        let age_metrics = map.snapshot_age_seconds(now);
        assert!(age_metrics.is_none());

        map.snapshot_timestamp_parse_errors_total()
            .expect("nonempty")
            .assert_contains_lines(&[
                "kopia_snapshot_timestamp_parse_errors_total{source=\"user_name@host:/path\"} 1",
            ]);
    }

    #[test]
    fn snapshot_age_multi_source() {
        use jiff::ToSpan as _;

        let now = jiff::Timestamp::now();
        let age1 = 45.minutes();
        let age2 = 120.minutes();

        let mut snapshot1 = test_snapshot("1", 1000, &["latest-1"]);
        snapshot1.end_time = (now - age1).to_string();

        let mut snapshot2 = test_snapshot("2", 2000, &["latest-1"]);
        snapshot2.end_time = (now - age2).to_string();

        let (map, _sources) = multi_map(vec![
            ("alice", "hostA", "/data", vec![snapshot1]),
            ("bob", "hostB", "/backup", vec![snapshot2]),
        ]);

        map.snapshot_age_seconds(now)
            .expect("nonempty")
            .assert_contains_snippets(&["# HELP kopia_snapshot_age_seconds"])
            .assert_contains_lines(&[
                "# TYPE kopia_snapshot_age_seconds gauge",
                "kopia_snapshot_age_seconds{source=\"alice@hostA:/data\"} 2700",
                "kopia_snapshot_age_seconds{source=\"bob@hostB:/backup\"} 7200",
            ]);
    }

    #[test]
    fn snapshot_timestamp_parse_errors_multi_source() {
        let mut snapshot1 = test_snapshot("1", 1000, &["latest-1"]);
        snapshot1.end_time = "invalid-time".to_string();

        let mut snapshot2 = test_snapshot("2", 2000, &["latest-1"]);
        snapshot2.end_time = "also-invalid".to_string();

        let mut snapshot3 = test_snapshot("3", 3000, &["latest-1"]);
        snapshot3.end_time = "still-invalid".to_string();

        let (map, _sources) = multi_map(vec![
            ("alice", "hostA", "/data", vec![snapshot1, snapshot2]),
            ("bob", "hostB", "/backup", vec![snapshot3]),
        ]);

        map.snapshot_timestamp_parse_errors_total()
            .expect("nonempty")
            .assert_contains_snippets(&["# HELP kopia_snapshot_timestamp_parse_errors_total"])
            .assert_contains_lines(&[
                "# TYPE kopia_snapshot_timestamp_parse_errors_total gauge",
                "kopia_snapshot_timestamp_parse_errors_total{source=\"alice@hostA:/data\"} 2",
                "kopia_snapshot_timestamp_parse_errors_total{source=\"bob@hostB:/backup\"} 1",
            ]);
    }
}
