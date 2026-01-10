use crate::{KopiaSnapshots, Snapshot, SourceMap, metrics::MetricLabel};
use std::fmt::{self, Display};

impl KopiaSnapshots {
    /// Generates Prometheus metrics for the total number of snapshots.
    ///
    /// Returns a string containing Prometheus-formatted metrics showing the total
    /// count of all snapshots in the repository.
    #[must_use]
    pub(super) fn snapshots_total(&self) -> impl Display {
        const NAME: &str = "kopia_snapshots_total";
        const LABEL: MetricLabel = MetricLabel::gauge(NAME, "Total number of snapshots");

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

        let Self { snapshots_map } = self;
        Output { snapshots_map }
    }
}

#[cfg(test)]
mod tests {
    use crate::test_util::{create_test_snapshot, single_map};

    #[test]
    fn snapshots_total_metrics() {
        let snapshots = vec![
            create_test_snapshot("1", 1000, &["latest-1"]),
            create_test_snapshot("2", 2000, &["daily-1"]),
            create_test_snapshot("3", 3000, &["monthly-1"]),
        ];

        let (map, _source) = single_map(snapshots);
        let metrics = map.snapshots_total().to_string();

        assert!(metrics.contains("# HELP kopia_snapshots_total"));
        assert!(metrics.contains("# TYPE kopia_snapshots_total gauge"));
        assert!(metrics.contains("kopia_snapshots_total{source=\"user_name@host:/path\"} 3"));
    }

    #[test]
    fn snapshots_total_metrics_empty() {
        let snapshots = vec![];
        let (map, _source) = single_map(snapshots);
        let metrics = map.snapshots_total().to_string();

        assert!(metrics.contains("kopia_snapshots_total{source=\"user_name@host:/path\"} 0"));
    }

    #[test]
    fn snapshots_total_metrics_single() {
        let snapshots = vec![create_test_snapshot("1", 1000, &["latest-1"])];
        let (map, _source) = single_map(snapshots);
        let metrics = map.snapshots_total().to_string();

        assert!(metrics.contains("kopia_snapshots_total{source=\"user_name@host:/path\"} 1"));
    }
}
