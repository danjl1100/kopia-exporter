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
    use crate::test_util::{single_map, test_snapshot};

    #[test]
    fn latest_snapshot_size_metrics() {
        let (map, _source) = single_map(vec![
            test_snapshot("1", 1000, &["daily-2"]),
            test_snapshot("2", 2000, &["latest-1"]),
        ]);

        let metrics = map
            .snapshot_total_size_bytes()
            .expect("nonempty")
            .to_string();

        assert!(metrics.contains("# HELP kopia_snapshot_total_size_bytes"));
        assert!(metrics.contains("# TYPE kopia_snapshot_total_size_bytes gauge"));
        assert!(
            metrics
                .contains("kopia_snapshot_total_size_bytes{source=\"user_name@host:/path\"} 2000"),
            "{metrics:?}"
        );
    }

    #[test]
    fn latest_snapshot_size_metrics_empty() {
        let (map, _source) = single_map(vec![]);
        let metrics = map.snapshot_total_size_bytes();

        assert!(metrics.is_none());
    }
}
