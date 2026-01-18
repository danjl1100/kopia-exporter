//! **New snapshot health:** Unix timestamp of last successful snapshot

use crate::{KopiaSnapshots, SourceMap, metrics::DisplayMetric};
use std::fmt::{self};

pub(super) struct SnapshotLastSuccessTimestamp(SourceMap<i64>);
impl DisplayMetric for SnapshotLastSuccessTimestamp {
    fn fmt(&self, name: &str, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Self(timestamps) = self;
        for (source, timestamp) in timestamps {
            writeln!(f, "{name}{{source={source:?}}} {timestamp}")?;
        }
        Ok(())
    }
}
impl SnapshotLastSuccessTimestamp {
    pub(super) fn new(ks: &KopiaSnapshots) -> Option<Self> {
        let timestamps: SourceMap<i64> = ks
            .snapshots_map
            .iter()
            .filter_map(|(source, snapshots)| {
                let last = snapshots.last()?;
                let end_time = last.end_time?;
                Some((source.clone(), end_time.as_second()))
            })
            .collect();

        timestamps.map_nonempty(Self)
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        AssertContains as _,
        test_util::{multi_map, single_map, test_snapshot},
    };

    #[test]
    fn snapshot_last_success_timestamp_metrics() {
        let mut snapshot1 = test_snapshot("1", 1000, &["daily-2"]);
        snapshot1.end_time = "2025-01-01T00:00:00Z".to_string();

        let mut snapshot2 = test_snapshot("2", 2000, &["latest-1"]);
        snapshot2.end_time = "2025-01-02T12:30:00Z".to_string();

        let (map, _source) = single_map(vec![snapshot1, snapshot2]);

        let expected_timestamp: i64 = "2025-01-02T12:30:00Z"
            .parse::<jiff::Timestamp>()
            .expect("valid timestamp")
            .as_second();

        map.kopia_snapshot_last_success_timestamp()
            .expect("nonempty")
            .assert_contains_snippets(&["# HELP kopia_snapshot_last_success_timestamp"])
            .assert_contains_lines(&[
                "# TYPE kopia_snapshot_last_success_timestamp gauge",
                &format!("kopia_snapshot_last_success_timestamp{{source=\"user_name@host:/path\"}} {expected_timestamp}"),
            ]);
    }

    #[test]
    fn snapshot_last_success_timestamp_multi_source() {
        let mut snapshot1 = test_snapshot("1", 1000, &["latest-1"]);
        snapshot1.end_time = "2025-01-01T10:00:00Z".to_string();

        let mut snapshot2 = test_snapshot("2", 2000, &["latest-1"]);
        snapshot2.end_time = "2025-01-02T15:30:00Z".to_string();

        let (map, _sources) = multi_map(vec![
            ("alice", "hostA", "/data", vec![snapshot1]),
            ("bob", "hostB", "/backup", vec![snapshot2]),
        ]);

        let timestamp1: i64 = "2025-01-01T10:00:00Z"
            .parse::<jiff::Timestamp>()
            .expect("valid timestamp")
            .as_second();
        let timestamp2: i64 = "2025-01-02T15:30:00Z"
            .parse::<jiff::Timestamp>()
            .expect("valid timestamp")
            .as_second();

        map.kopia_snapshot_last_success_timestamp()
            .expect("nonempty")
            .assert_contains_snippets(&["# HELP kopia_snapshot_last_success_timestamp"])
            .assert_contains_lines(&[
                "# TYPE kopia_snapshot_last_success_timestamp gauge",
                &format!("kopia_snapshot_last_success_timestamp{{source=\"alice@hostA:/data\"}} {timestamp1}"),
                &format!("kopia_snapshot_last_success_timestamp{{source=\"bob@hostB:/backup\"}} {timestamp2}"),
            ]);
    }

    #[test]
    fn snapshot_last_success_timestamp_metrics_empty() {
        let (map, _source) = single_map(vec![]);
        let metrics = map.kopia_snapshot_last_success_timestamp();

        assert!(metrics.is_none());
    }

    #[test]
    fn snapshot_last_success_timestamp_invalid_time() {
        let mut snapshot = test_snapshot("1", 1000, &["latest-1"]);
        snapshot.end_time = "invalid-time".to_string();

        let (map, _source) = single_map(vec![snapshot]);
        let metrics = map.kopia_snapshot_last_success_timestamp();

        assert!(metrics.is_none());
    }
}
