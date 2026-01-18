//! **Backup completion status:** Total errors in latest snapshot

#[cfg(test)]
mod tests {
    use crate::{
        AssertContains as _,
        test_util::{multi_map, single_map, test_snapshot},
    };

    #[test]
    fn snapshot_errors() {
        let mut snapshot = test_snapshot("1", 1000, &["latest-1"]);
        snapshot.stats.error_count = 5;

        let (map, _source) = single_map(vec![snapshot]);
        map.kopia_snapshot_errors_total()
            .expect("nonempty")
            .assert_contains_snippets(&["# HELP kopia_snapshot_errors_total"])
            .assert_contains_lines(&[
                "# TYPE kopia_snapshot_errors_total gauge",
                "kopia_snapshot_errors_total{source=\"user_name@host:/path\"} 5",
            ]);
    }

    #[test]
    fn snapshot_errors_no_errors() {
        let snapshot = test_snapshot("1", 1000, &["latest-1"]);

        let (map, _source) = single_map(vec![snapshot]);
        map.kopia_snapshot_errors_total()
            .expect("nonempty")
            .assert_contains_lines(&[
                "kopia_snapshot_errors_total{source=\"user_name@host:/path\"} 0",
            ]);
    }

    #[test]
    fn snapshot_errors_metrics_empty() {
        let snapshots = vec![];
        let (map, _source) = single_map(snapshots);
        let metrics = map.kopia_snapshot_errors_total();

        assert!(metrics.is_none());
    }

    #[test]
    fn snapshot_errors_multi_source() {
        let mut snapshot1 = test_snapshot("1", 1000, &["latest-1"]);
        snapshot1.stats.error_count = 7;

        let mut snapshot2 = test_snapshot("2", 2000, &["latest-1"]);
        snapshot2.stats.error_count = 3;

        let (map, _sources) = multi_map(vec![
            ("alice", "hostA", "/data", vec![snapshot1]),
            ("bob", "hostB", "/backup", vec![snapshot2]),
        ]);

        map.kopia_snapshot_errors_total()
            .expect("nonempty")
            .assert_contains_snippets(&["# HELP kopia_snapshot_errors_total"])
            .assert_contains_lines(&[
                "# TYPE kopia_snapshot_errors_total gauge",
                "kopia_snapshot_errors_total{source=\"alice@hostA:/data\"} 7",
                "kopia_snapshot_errors_total{source=\"bob@hostB:/backup\"} 3",
            ]);
    }
}
