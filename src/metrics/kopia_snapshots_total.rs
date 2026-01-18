use crate::{KopiaSnapshots, Snapshot, SourceMap, metrics::DisplayMetric};
use std::fmt;

pub(super) struct SnapshotsTotal<'a> {
    snapshots_map: &'a SourceMap<Vec<Snapshot>>,
}
impl DisplayMetric for SnapshotsTotal<'_> {
    fn fmt(&self, name: &str, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Self { snapshots_map } = *self;
        for (source, snapshots) in snapshots_map {
            let count = snapshots.len();
            writeln!(f, "{name}{{source={source:?}}} {count}")?;
        }
        Ok(())
    }
}

impl<'a> SnapshotsTotal<'a> {
    pub fn new(ks: &'a KopiaSnapshots) -> Self {
        let KopiaSnapshots { snapshots_map, .. } = ks;
        Self { snapshots_map }
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        AssertContains as _,
        test_util::{multi_map, single_map, test_snapshot},
    };

    #[test]
    fn snapshots_total_metrics() {
        let snapshots = vec![
            test_snapshot("1", 1000, &["latest-1"]),
            test_snapshot("2", 2000, &["daily-1"]),
            test_snapshot("3", 3000, &["monthly-1"]),
        ];

        let (map, _source) = single_map(snapshots);
        map.kopia_snapshots_total()
            .assert_contains_snippets(&["# HELP kopia_snapshots_total"])
            .assert_contains_lines(&[
                "# TYPE kopia_snapshots_total gauge",
                "kopia_snapshots_total{source=\"user_name@host:/path\"} 3",
            ]);
    }

    #[test]
    fn snapshots_total_metrics_empty() {
        let snapshots = vec![];
        let (map, _source) = single_map(snapshots);
        let metrics = map.kopia_snapshots_total().to_string();

        insta::assert_snapshot!(metrics, @r"
        # HELP kopia_snapshots_total Total number of snapshots
        # TYPE kopia_snapshots_total gauge
        ");
    }

    #[test]
    fn snapshots_total_metrics_single() {
        let snapshots = vec![test_snapshot("1", 1000, &["latest-1"])];
        let (map, _source) = single_map(snapshots);
        map.kopia_snapshots_total()
            .assert_contains_lines(&["kopia_snapshots_total{source=\"user_name@host:/path\"} 1"]);
    }

    #[test]
    fn snapshots_total_multi_source() {
        let snapshots_1 = vec![
            test_snapshot("1", 1000, &["latest-1"]),
            test_snapshot("2", 2000, &["daily-1"]),
        ];
        let snapshots_2 = vec![
            test_snapshot("3", 3000, &["latest-1"]),
            test_snapshot("4", 4000, &["daily-1"]),
            test_snapshot("5", 5000, &["monthly-1"]),
        ];
        let (map, _sources) = multi_map(vec![
            ("alice", "hostA", "/data", snapshots_1),
            ("bob", "hostB", "/backup", snapshots_2),
        ]);

        map.kopia_snapshots_total()
            .assert_contains_snippets(&["# HELP kopia_snapshots_total"])
            .assert_contains_lines(&[
                "# TYPE kopia_snapshots_total gauge",
                "kopia_snapshots_total{source=\"alice@hostA:/data\"} 2",
                "kopia_snapshots_total{source=\"bob@hostB:/backup\"} 3",
            ]);
    }
}
