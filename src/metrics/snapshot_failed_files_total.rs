//! **Data integrity verification:** Number of failed files in latest snapshot

use crate::{KopiaSnapshots, metrics::last_snapshots::MetricLastSnapshots};
use std::fmt::Display;

crate::define_metric! {
    name: "kopia_snapshot_failed_files_total",
    help: "Number of failed files in latest snapshot",
    category: "Data integrity verification",
    type: Gauge,
}

impl KopiaSnapshots {
    /// Generates Prometheus metrics for failed files in the latest snapshot.
    ///
    /// Returns a string containing Prometheus-formatted metrics showing the number
    /// of failed files in the most recent snapshot. Only present if snapshots list is not empty.
    #[must_use]
    pub(super) fn snapshot_failed_files_total(&self) -> Option<impl Display> {
        MetricLastSnapshots::new(self, NAME, LABEL, |v| v.root_entry.summ.num_failed)
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        AssertContains as _,
        test_util::{multi_map, single_map, test_snapshot},
    };

    #[test]
    fn snapshot_failed_files_metrics() {
        let mut snapshot = test_snapshot("1", 1000, &["latest-1"]);
        snapshot.root_entry.summ.num_failed = 3;

        let (map, _source) = single_map(vec![snapshot]);
        map.snapshot_failed_files_total()
            .expect("nonempty")
            .assert_contains_snippets(&["# HELP kopia_snapshot_failed_files_total"])
            .assert_contains_lines(&[
                "# TYPE kopia_snapshot_failed_files_total gauge",
                "kopia_snapshot_failed_files_total{source=\"user_name@host:/path\"} 3",
            ]);
    }

    #[test]
    fn snapshot_failed_files_metrics_no_failures() {
        let snapshot = test_snapshot("1", 1000, &["latest-1"]);

        let (map, _source) = single_map(vec![snapshot]);
        map.snapshot_failed_files_total()
            .expect("nonempty")
            .assert_contains_lines(&[
                "kopia_snapshot_failed_files_total{source=\"user_name@host:/path\"} 0",
            ]);
    }

    #[test]
    fn snapshot_failed_files_metrics_empty() {
        let snapshots = vec![];
        let (map, _source) = &single_map(snapshots);
        let metrics = map.snapshot_failed_files_total();

        assert!(metrics.is_none());
    }

    #[test]
    fn snapshot_failed_files_multi_source() {
        let mut snapshot1 = test_snapshot("1", 1000, &["latest-1"]);
        snapshot1.root_entry.summ.num_failed = 5;

        let mut snapshot2 = test_snapshot("2", 2000, &["latest-1"]);
        snapshot2.root_entry.summ.num_failed = 2;

        let (map, _sources) = multi_map(vec![
            ("alice", "hostA", "/data", vec![snapshot1]),
            ("bob", "hostB", "/backup", vec![snapshot2]),
        ]);

        map.snapshot_failed_files_total()
            .expect("nonempty")
            .assert_contains_snippets(&["# HELP kopia_snapshot_failed_files_total"])
            .assert_contains_lines(&[
                "# TYPE kopia_snapshot_failed_files_total gauge",
                "kopia_snapshot_failed_files_total{source=\"alice@hostA:/data\"} 5",
                "kopia_snapshot_failed_files_total{source=\"bob@hostB:/backup\"} 2",
            ]);
    }
}
