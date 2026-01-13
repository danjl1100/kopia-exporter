//! **Pruned snapshots:** Number of snapshots by retention reason

use crate::{KopiaSnapshots, SourceMap};
use std::{
    collections::BTreeMap,
    fmt::{self, Display},
};

crate::define_metric! {
    name: "kopia_snapshots_by_retention",
    help: "Number of snapshots by retention reason",
    category: "Pruned snapshots",
    type: Gauge,
}

impl KopiaSnapshots {
    /// Generates Prometheus metrics for snapshots by retention reason.
    ///
    /// Returns a string containing Prometheus-formatted metrics showing the count
    /// of snapshots for each retention reason (e.g., "latest-1", "daily-7", etc.).
    #[must_use]
    pub(super) fn snapshots_by_retention(&self) -> impl Display {
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
    use crate::{
        AssertContains as _,
        test_util::{multi_map, single_map, test_snapshot},
    };

    #[test]
    fn snapshots_by_retention_metrics() {
        let (map, _source) = &single_map(vec![
            test_snapshot("1", 1000, &["latest-1", "daily-1"]),
            test_snapshot("2", 2000, &["daily-2"]),
        ]);

        map.snapshots_by_retention()
            .assert_contains_snippets(&["# HELP kopia_snapshots_by_retention"])
            .assert_contains_lines(&[
                "# TYPE kopia_snapshots_by_retention gauge",
                "kopia_snapshots_by_retention{source=\"user_name@host:/path\",retention_reason=\"latest-1\"} 1",
                "kopia_snapshots_by_retention{source=\"user_name@host:/path\",retention_reason=\"daily-1\"} 1",
                "kopia_snapshots_by_retention{source=\"user_name@host:/path\",retention_reason=\"daily-2\"} 1",
            ]);
    }

    #[test]
    fn snapshots_by_retention_multi_source() {
        let snapshots_1 = vec![
            test_snapshot("1", 1000, &["latest-1", "daily-1"]),
            test_snapshot("2", 2000, &["daily-2"]),
        ];
        let snapshots_2 = vec![
            test_snapshot("3", 3000, &["latest-1"]),
            test_snapshot("4", 4000, &["latest-1", "monthly-1"]),
            test_snapshot("5", 5000, &["daily-1"]),
        ];
        let (map, _sources) = multi_map(vec![
            ("alice", "hostA", "/data", snapshots_1),
            ("bob", "hostB", "/backup", snapshots_2),
        ]);

        map.snapshots_by_retention()
            .assert_contains_snippets(&["# HELP kopia_snapshots_by_retention"])
            .assert_contains_lines(&[
                "# TYPE kopia_snapshots_by_retention gauge",
                "kopia_snapshots_by_retention{source=\"alice@hostA:/data\",retention_reason=\"latest-1\"} 1",
                "kopia_snapshots_by_retention{source=\"alice@hostA:/data\",retention_reason=\"daily-1\"} 1",
                "kopia_snapshots_by_retention{source=\"alice@hostA:/data\",retention_reason=\"daily-2\"} 1",
                "kopia_snapshots_by_retention{source=\"bob@hostB:/backup\",retention_reason=\"latest-1\"} 2",
                "kopia_snapshots_by_retention{source=\"bob@hostB:/backup\",retention_reason=\"monthly-1\"} 1",
                "kopia_snapshots_by_retention{source=\"bob@hostB:/backup\",retention_reason=\"daily-1\"} 1",
            ]);
    }
}
