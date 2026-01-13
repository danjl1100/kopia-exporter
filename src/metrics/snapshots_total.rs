//! **Pruned snapshots:** Total number of snapshots

use crate::{KopiaSnapshots, Snapshot, SourceMap};
use std::fmt::{self, Display};

crate::define_metric! {
    name: "kopia_snapshots_total",
    help: "Total number of snapshots",
    category: "Pruned snapshots",
    type: Gauge,
}

impl KopiaSnapshots {
    /// Generates Prometheus metrics for the total number of snapshots.
    ///
    /// Returns a string containing Prometheus-formatted metrics showing the total
    /// count of all snapshots in the repository.
    #[must_use]
    pub(super) fn snapshots_total(&self) -> impl Display {
        struct Output<'a> {
            snapshots_map: &'a SourceMap<Vec<Snapshot>>,
        }
        impl Display for Output<'_> {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                let Self { snapshots_map } = *self;
                writeln!(f, "{LABEL}")?;
                for (source, snapshots) in snapshots_map {
                    let count = snapshots.len();
                    writeln!(f, "{NAME}{{source={source:?}}} {count}")?;
                }
                Ok(())
            }
        }

        let Self { snapshots_map, .. } = self;
        Output { snapshots_map }
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
        map.snapshots_total()
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
        let metrics = map.snapshots_total().to_string();

        insta::assert_snapshot!(metrics, @r"
        # HELP kopia_snapshots_total Total number of snapshots
        # TYPE kopia_snapshots_total gauge
        ");
    }

    #[test]
    fn snapshots_total_metrics_single() {
        let snapshots = vec![test_snapshot("1", 1000, &["latest-1"])];
        let (map, _source) = single_map(snapshots);
        map.snapshots_total()
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

        map.snapshots_total()
            .assert_contains_snippets(&["# HELP kopia_snapshots_total"])
            .assert_contains_lines(&[
                "# TYPE kopia_snapshots_total gauge",
                "kopia_snapshots_total{source=\"alice@hostA:/data\"} 2",
                "kopia_snapshots_total{source=\"bob@hostB:/backup\"} 3",
            ]);
    }
}
