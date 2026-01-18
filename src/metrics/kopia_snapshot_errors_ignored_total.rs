#[cfg(test)]
mod tests {
    use crate::{
        AssertContains as _,
        test_util::{multi_map, single_map, test_snapshot},
    };

    #[test]
    fn latest_snapshot_ignored_errors_metrics() {
        let mut snap1 = test_snapshot("1", 1000, &["daily-2"]);
        snap1.stats.ignored_error_count = 5;

        let mut snap2 = test_snapshot("2", 2000, &["latest-1"]);
        snap2.stats.ignored_error_count = 3;

        let (map, _source) = single_map(vec![snap1, snap2]);

        map.kopia_snapshot_errors_ignored_total()
            .expect("nonempty")
            .assert_contains_snippets(&["# HELP kopia_snapshot_errors_ignored_total"])
            .assert_contains_lines(&[
                "# TYPE kopia_snapshot_errors_ignored_total gauge",
                "kopia_snapshot_errors_ignored_total{source=\"user_name@host:/path\"} 3",
            ]);
    }

    #[test]
    fn latest_snapshot_ignored_errors_metrics_empty() {
        let (map, _source) = single_map(vec![]);
        let metrics = map.kopia_snapshot_errors_ignored_total();

        assert!(metrics.is_none());
    }

    #[test]
    fn latest_snapshot_ignored_errors_multi_source() {
        let mut snapshot1 = test_snapshot("1", 1000, &["latest-1"]);
        snapshot1.stats.ignored_error_count = 4;

        let mut snapshot2 = test_snapshot("2", 2000, &["latest-1"]);
        snapshot2.stats.ignored_error_count = 1;

        let (map, _sources) = multi_map(vec![
            ("alice", "hostA", "/data", vec![snapshot1]),
            ("bob", "hostB", "/backup", vec![snapshot2]),
        ]);

        map.kopia_snapshot_errors_ignored_total()
            .expect("nonempty")
            .assert_contains_snippets(&["# HELP kopia_snapshot_errors_ignored_total"])
            .assert_contains_lines(&[
                "# TYPE kopia_snapshot_errors_ignored_total gauge",
                "kopia_snapshot_errors_ignored_total{source=\"alice@hostA:/data\"} 4",
                "kopia_snapshot_errors_ignored_total{source=\"bob@hostB:/backup\"} 1",
            ]);
    }
}
