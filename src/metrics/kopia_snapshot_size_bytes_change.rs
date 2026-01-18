use crate::{KopiaSnapshots, SourceMap, metrics::DisplayMetric};
use std::fmt;

pub(super) struct SnapshotSizeByteChanges(SourceMap<i128>);
impl DisplayMetric for SnapshotSizeByteChanges {
    fn fmt(&self, name: &str, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Self(size_changes) = self;
        for (source, size_change) in size_changes {
            writeln!(f, "{name}{{source={source:?}}} {size_change}")?;
        }
        Ok(())
    }
}

impl SnapshotSizeByteChanges {
    pub fn new(ks: &KopiaSnapshots) -> Option<Self> {
        let size_changes: SourceMap<i128> = ks
            .snapshots_map
            .iter()
            .filter_map(|(source, snapshots)| {
                let mut iter = snapshots.iter().rev();
                let latest = iter.next()?;
                let previous = iter.next()?;

                let latest_size: u64 = latest.stats.total_size;
                let previous_size: u64 = previous.stats.total_size;

                let size_change = u128::from(latest_size)
                    .checked_signed_diff(u128::from(previous_size))
                    .expect("u64 diff fits in i128");
                Some((source.clone(), size_change))
            })
            .collect();
        size_changes.map_nonempty(Self)
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        AssertContains as _,
        test_util::{multi_map, single_map, test_snapshot},
    };

    #[test]
    fn snapshot_size_change_positive() {
        let (map, _source) = single_map(vec![
            test_snapshot("1", 1000, &["daily-2"]),
            test_snapshot("2", 2500, &["latest-1"]),
        ]);

        map.kopia_snapshot_size_bytes_change()
            .expect("nonempty")
            .assert_contains_snippets(&["# HELP kopia_snapshot_size_bytes_change"])
            .assert_contains_lines(&[
                "# TYPE kopia_snapshot_size_bytes_change gauge",
                "kopia_snapshot_size_bytes_change{source=\"user_name@host:/path\"} 1500",
            ]);
    }

    #[test]
    fn snapshot_size_change_negative() {
        let (map, _source) = single_map(vec![
            test_snapshot("1", 5000, &["daily-2"]),
            test_snapshot("2", 2000, &["latest-1"]),
        ]);

        map.kopia_snapshot_size_bytes_change()
            .expect("nonempty")
            .assert_contains_lines(&[
                "kopia_snapshot_size_bytes_change{source=\"user_name@host:/path\"} -3000",
            ]);
    }

    #[test]
    fn snapshot_size_change_single_snapshot() {
        let (map, _source) = single_map(vec![test_snapshot("1", 1000, &["latest-1"])]);

        let metrics = map.kopia_snapshot_size_bytes_change();
        assert!(metrics.is_none());
    }

    #[test]
    fn snapshot_size_change_empty() {
        let (map, _source) = single_map(vec![]);

        let metrics = map.kopia_snapshot_size_bytes_change();
        assert!(metrics.is_none());
    }

    #[test]
    fn snapshot_size_change_multi_source() {
        let snapshots_1 = vec![
            test_snapshot("1", 1000, &["daily-2"]),
            test_snapshot("2", 3500, &["latest-1"]),
        ];
        let snapshots_2 = vec![
            test_snapshot("3", 8000, &["daily-2"]),
            test_snapshot("4", 5000, &["latest-1"]),
        ];
        let (map, _sources) = multi_map(vec![
            ("alice", "hostA", "/data", snapshots_1),
            ("bob", "hostB", "/backup", snapshots_2),
        ]);

        map.kopia_snapshot_size_bytes_change()
            .expect("nonempty")
            .assert_contains_snippets(&["# HELP kopia_snapshot_size_bytes_change"])
            .assert_contains_lines(&[
                "# TYPE kopia_snapshot_size_bytes_change gauge",
                "kopia_snapshot_size_bytes_change{source=\"alice@hostA:/data\"} 2500",
                "kopia_snapshot_size_bytes_change{source=\"bob@hostB:/backup\"} -3000",
            ]);
    }
}
