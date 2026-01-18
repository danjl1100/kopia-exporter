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

        map.kopia_snapshot_size_bytes_total()
            .expect("nonempty")
            .assert_contains_snippets(&["# HELP kopia_snapshot_size_bytes_total"])
            .assert_contains_lines(&[
                "# TYPE kopia_snapshot_size_bytes_total gauge",
                "kopia_snapshot_size_bytes_total{source=\"user_name@host:/path\"} 2000",
            ]);
    }

    #[test]
    fn latest_snapshot_size_metrics_empty() {
        let (map, _source) = single_map(vec![]);
        let metrics = map.kopia_snapshot_size_bytes_total();

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

        map.kopia_snapshot_size_bytes_total()
            .expect("nonempty")
            .assert_contains_snippets(&["# HELP kopia_snapshot_size_bytes_total"])
            .assert_contains_lines(&[
                "# TYPE kopia_snapshot_size_bytes_total gauge",
                "kopia_snapshot_size_bytes_total{source=\"alice@hostA:/data\"} 2500",
                "kopia_snapshot_size_bytes_total{source=\"bob@hostB:/backup\"} 8000",
            ]);
    }
}
