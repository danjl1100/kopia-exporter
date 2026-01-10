use crate::{
    KopiaSnapshots,
    metrics::{MetricLabel, last_snapshots::MetricLastSnapshots},
};
use std::fmt::Display;

impl KopiaSnapshots {
    /// Generates Prometheus metrics for ignored errors in the latest snapshot.
    ///
    /// Returns a string containing Prometheus-formatted metrics showing the total
    /// number of ignored errors in the most recent snapshot. Only present if snapshots list is not empty.
    #[must_use]
    pub(super) fn snapshot_ignored_errors_total(&self) -> Option<impl Display> {
        const NAME: &str = "kopia_snapshot_ignored_errors_total";
        const LABEL: MetricLabel = MetricLabel::gauge(NAME, "Ignored errors in latest snapshot");

        MetricLastSnapshots::new(self, NAME, LABEL, |v| v.stats.ignored_error_count)
    }
}

#[cfg(test)]
mod tests {
    use crate::test_util::{single_map, test_snapshot};

    #[test]
    fn latest_snapshot_ignored_errors_metrics() {
        let mut snap1 = test_snapshot("1", 1000, &["daily-2"]);
        snap1.stats.ignored_error_count = 5;

        let mut snap2 = test_snapshot("2", 2000, &["latest-1"]);
        snap2.stats.ignored_error_count = 3;

        let (map, _source) = single_map(vec![snap1, snap2]);

        let metrics = map
            .snapshot_ignored_errors_total()
            .expect("nonempty")
            .to_string();

        assert!(metrics.contains("# HELP kopia_snapshot_ignored_errors_total"));
        assert!(metrics.contains("# TYPE kopia_snapshot_ignored_errors_total gauge"));
        assert!(
            metrics
                .contains("kopia_snapshot_ignored_errors_total{source=\"user_name@host:/path\"} 3"),
            "{metrics:?}"
        );
    }

    #[test]
    fn latest_snapshot_ignored_errors_metrics_empty() {
        let (map, _source) = single_map(vec![]);
        let metrics = map.snapshot_ignored_errors_total();

        assert!(metrics.is_none());
    }
}
