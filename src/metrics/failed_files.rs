use crate::{
    KopiaSnapshots,
    metrics::{MetricLabel, last_snapshots::MetricLastSnapshots},
};
use std::fmt::Display;

impl KopiaSnapshots {
    /// Generates Prometheus metrics for failed files in the latest snapshot.
    ///
    /// Returns a string containing Prometheus-formatted metrics showing the number
    /// of failed files in the most recent snapshot. Only present if snapshots list is not empty.
    #[must_use]
    pub(super) fn snapshot_failed_files_total(&self) -> Option<impl Display> {
        const NAME: &str = "kopia_snapshot_failed_files_total";
        const LABEL: MetricLabel =
            MetricLabel::gauge(NAME, "Number of failed files in latest snapshot");

        MetricLastSnapshots::new(self, NAME, LABEL, |v| v.root_entry.summ.num_failed)
    }
}

#[cfg(test)]
mod tests {
    use crate::test_util::{single_map, test_snapshot};

    #[test]
    fn snapshot_failed_files_metrics() {
        let mut snapshot = test_snapshot("1", 1000, &["latest-1"]);
        snapshot.root_entry.summ.num_failed = 3;

        let (map, _source) = single_map(vec![snapshot]);
        let metrics = map
            .snapshot_failed_files_total()
            .expect("nonempty")
            .to_string();

        assert!(metrics.contains("# HELP kopia_snapshot_failed_files_total"));
        assert!(metrics.contains("# TYPE kopia_snapshot_failed_files_total gauge"));
        assert!(
            metrics
                .contains("kopia_snapshot_failed_files_total{source=\"user_name@host:/path\"} 3")
        );
    }

    #[test]
    fn snapshot_failed_files_metrics_no_failures() {
        let snapshot = test_snapshot("1", 1000, &["latest-1"]);

        let (map, _source) = single_map(vec![snapshot]);
        let metrics = map
            .snapshot_failed_files_total()
            .expect("nonempty")
            .to_string();

        assert!(
            metrics
                .contains("kopia_snapshot_failed_files_total{source=\"user_name@host:/path\"} 0")
        );
    }

    #[test]
    fn snapshot_failed_files_metrics_empty() {
        let snapshots = vec![];
        let (map, _source) = &single_map(snapshots);
        let metrics = map.snapshot_failed_files_total();

        assert!(metrics.is_none());
    }
}
