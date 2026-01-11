use crate::{
    KopiaSnapshots,
    metrics::{MetricLabel, last_snapshots::MetricLastSnapshots},
};
use std::fmt::Display;

impl KopiaSnapshots {
    /// Generates Prometheus metrics for the latest snapshot size.
    ///
    /// Returns a string containing Prometheus-formatted metrics showing the total
    /// size in bytes of the most recent snapshot. Only present if snapshots list is not empty.
    #[must_use]
    pub(super) fn snapshot_total_size_bytes(&self) -> Option<impl Display> {
        const NAME: &str = "kopia_snapshot_total_size_bytes";
        const LABEL: MetricLabel =
            MetricLabel::gauge(NAME, "Total size of latest snapshot in bytes");

        MetricLastSnapshots::new(self, NAME, LABEL, |v| v.stats.total_size)
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        AssertContains as _,
        test_util::{multi_map, single_map, test_snapshot},
    };

    #[test]
    fn latest_snapshot_size_metrics() {
        let (map, _source) = single_map(vec![
            test_snapshot("1", 1000, &["daily-2"]),
            test_snapshot("2", 2000, &["latest-1"]),
        ]);

        map.snapshot_total_size_bytes()
            .expect("nonempty")
            .assert_contains_snippets(&["# HELP kopia_snapshot_total_size_bytes"])
            .assert_contains_lines(&[
                "# TYPE kopia_snapshot_total_size_bytes gauge",
                "kopia_snapshot_total_size_bytes{source=\"user_name@host:/path\"} 2000",
            ]);
    }

    #[test]
    fn latest_snapshot_size_metrics_empty() {
        let (map, _source) = single_map(vec![]);
        let metrics = map.snapshot_total_size_bytes();

        assert!(metrics.is_none());
    }

    #[test]
    fn latest_snapshot_size_multi_source() {
        let snapshots_1 = vec![
            test_snapshot("1", 1000, &["daily-2"]),
            test_snapshot("2", 2500, &["latest-1"]),
        ];
        let snapshots_2 = vec![
            test_snapshot("3", 5000, &["daily-2"]),
            test_snapshot("4", 8000, &["latest-1"]),
        ];
        let (map, _sources) = multi_map(vec![
            ("alice", "hostA", "/data", snapshots_1),
            ("bob", "hostB", "/backup", snapshots_2),
        ]);

        map.snapshot_total_size_bytes()
            .expect("nonempty")
            .assert_contains_snippets(&["# HELP kopia_snapshot_total_size_bytes"])
            .assert_contains_lines(&[
                "# TYPE kopia_snapshot_total_size_bytes gauge",
                "kopia_snapshot_total_size_bytes{source=\"alice@hostA:/data\"} 2500",
                "kopia_snapshot_total_size_bytes{source=\"bob@hostB:/backup\"} 8000",
            ]);
    }
}
