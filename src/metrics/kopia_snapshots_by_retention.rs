use crate::{KopiaSnapshots, SourceMap, metrics::DisplayMetric};
use std::{collections::BTreeMap, fmt};

pub(super) struct SnapshotsByRetention {
    retention_counts: SourceMap<BTreeMap<String, u32>>,
}
impl DisplayMetric for SnapshotsByRetention {
    fn fmt(&self, name: &str, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Self { retention_counts } = self;
        for (source, reason_counts) in retention_counts {
            for (reason, count) in reason_counts {
                writeln!(
                    f,
                    "{name}{{source={source:?},retention_reason={reason:?}}} {count}"
                )?;
            }
        }
        Ok(())
    }
}
impl SnapshotsByRetention {
    pub fn new(ks: &KopiaSnapshots) -> Self {
        let retention_counts = ks.get_retention_counts();
        SnapshotsByRetention { retention_counts }
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

        map.kopia_snapshots_by_retention()
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

        map.kopia_snapshots_by_retention()
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
