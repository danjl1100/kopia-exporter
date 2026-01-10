use crate::{KopiaSnapshots, SourceMap, metrics::MetricLabel};
use std::{
    collections::BTreeMap,
    fmt::{self, Display},
};

impl KopiaSnapshots {
    /// Generates Prometheus metrics for snapshots by retention reason.
    ///
    /// Returns a string containing Prometheus-formatted metrics showing the count
    /// of snapshots for each retention reason (e.g., "latest-1", "daily-7", etc.).
    #[must_use]
    pub(super) fn snapshots_by_retention(&self) -> impl Display {
        const NAME: &str = "kopia_snapshots_by_retention";
        const LABEL: MetricLabel =
            MetricLabel::gauge(NAME, "Number of snapshots by retention reason");

        struct Output {
            retention_counts: SourceMap<BTreeMap<String, u32>>,
        }
        impl Display for Output {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                let Self { retention_counts } = self;
                writeln!(f, "{LABEL}")?;
                for (source, reason_counts) in retention_counts {
                    for (reason, count) in reason_counts {
                        writeln!(
                            f,
                            "{NAME}{{source={source:?},retention_reason={reason:?}}} {count}"
                        )?;
                    }
                }
                Ok(())
            }
        }

        let retention_counts = self.get_retention_counts();
        Output { retention_counts }
    }
}

#[cfg(test)]
mod tests {
    use crate::test_util::{create_test_snapshot, single_map};

    #[test]
    fn snapshots_by_retention_metrics() {
        let (map, _source) = &single_map(vec![
            create_test_snapshot("1", 1000, &["latest-1", "daily-1"]),
            create_test_snapshot("2", 2000, &["daily-2"]),
        ]);

        let metrics = map.snapshots_by_retention().to_string();

        assert!(metrics.contains("# HELP kopia_snapshots_by_retention"));
        assert!(metrics.contains("# TYPE kopia_snapshots_by_retention gauge"));
        assert!(metrics.contains("kopia_snapshots_by_retention{source=\"user_name@host:/path\",retention_reason=\"latest-1\"} 1"));
        assert!(metrics.contains("kopia_snapshots_by_retention{source=\"user_name@host:/path\",retention_reason=\"daily-1\"} 1"));
        assert!(metrics.contains("kopia_snapshots_by_retention{source=\"user_name@host:/path\",retention_reason=\"daily-2\"} 1"));
    }
}
