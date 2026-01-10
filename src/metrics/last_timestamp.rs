use crate::{KopiaSnapshots, SourceMap, metrics::MetricLabel};
use std::fmt::{self, Display};

impl KopiaSnapshots {
    /// Generates Prometheus metrics for the last successful snapshot timestamp.
    ///
    /// Returns a string containing Prometheus-formatted metrics showing the Unix timestamp
    /// of the most recent snapshot. Only present if snapshots list is not empty.
    #[must_use]
    pub(super) fn snapshot_last_success_timestamp(&self) -> Option<impl Display> {
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

        let timestamps: SourceMap<i64> = self
            .snapshots_map
            .iter()
            .filter_map(|(source, snapshots)| {
                let last = snapshots.last()?;
                let end_time = last.end_time?;
                Some((source.clone(), end_time.as_second()))
            })
            .collect();

        timestamps.map_nonempty(Timestamps)
    }
}

#[cfg(test)]
mod tests {
    use crate::test_util::{create_test_snapshot_json, single_map};

    #[test]
    fn snapshot_last_success_timestamp_metrics() {
        let mut snapshot1 = create_test_snapshot_json("1", 1000, &["daily-2"]);
        snapshot1.end_time = "2025-01-01T00:00:00Z".to_string();

        let mut snapshot2 = create_test_snapshot_json("2", 2000, &["latest-1"]);
        snapshot2.end_time = "2025-01-02T12:30:00Z".to_string();

        let (map, _source) = single_map(vec![snapshot1.into(), snapshot2.into()]);

        let metrics = map
            .snapshot_last_success_timestamp()
            .expect("nonempty")
            .to_string();

        // Calculate expected timestamp for "2025-01-02T12:30:00Z"
        let expected_timestamp: i64 = "2025-01-02T12:30:00Z"
            .parse::<jiff::Timestamp>()
            .expect("valid timestamp")
            .as_second();

        assert!(metrics.contains("# HELP kopia_snapshot_last_success_timestamp"));
        assert!(metrics.contains("# TYPE kopia_snapshot_last_success_timestamp gauge"));
        assert!(
            metrics.contains(&format!(
                "kopia_snapshot_last_success_timestamp{{source=\"user_name@host:/path\"}} {expected_timestamp}"
            )),
            "{metrics:?}"
        );
    }

    #[test]
    fn snapshot_last_success_timestamp_metrics_empty() {
        let (map, _source) = single_map(vec![]);
        let metrics = map.snapshot_last_success_timestamp();

        assert!(metrics.is_none());
    }

    #[test]
    fn snapshot_last_success_timestamp_invalid_time() {
        let mut snapshot = create_test_snapshot_json("1", 1000, &["latest-1"]);
        snapshot.end_time = "invalid-time".to_string();

        let (map, _source) = single_map(vec![snapshot.into()]);
        let metrics = map.snapshot_last_success_timestamp();

        assert!(metrics.is_none());
    }
}
