use crate::{KopiaSnapshots, SourceMap, metrics::MetricLabel};
use std::fmt::{self, Display};

impl KopiaSnapshots {
    /// Generates Prometheus metrics for the size change from the previous snapshot.
    ///
    /// Returns a string containing Prometheus-formatted metrics showing the change
    /// in bytes from the previous snapshot. Only present if snapshots list has more than one snapshot.
    #[must_use]
    pub(super) fn snapshot_size_change_bytes(&self) -> Option<impl Display> {
        const NAME: &str = "kopia_snapshot_size_change_bytes";
        const LABEL: MetricLabel =
            MetricLabel::gauge(NAME, "Change in size from previous snapshot");

        struct SizeChanges(SourceMap<i128>);
        impl Display for SizeChanges {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                let Self(size_changes) = self;
                writeln!(f, "{LABEL}")?;
                for (source, size_change) in size_changes {
                    writeln!(f, "{NAME}{{source={source:?}}} {size_change}")?;
                }
                Ok(())
            }
        }

        let size_changes: SourceMap<i128> = self
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
        size_changes.map_nonempty(SizeChanges)
    }
}

#[cfg(test)]
mod tests {
    use crate::test_util::{single_map, test_snapshot};

    #[test]
    fn snapshot_size_change_positive() {
        let (map, _source) = single_map(vec![
            test_snapshot("1", 1000, &["daily-2"]),
            test_snapshot("2", 2500, &["latest-1"]),
        ]);

        let metrics = map
            .snapshot_size_change_bytes()
            .expect("nonempty")
            .to_string();

        assert!(metrics.contains("# HELP kopia_snapshot_size_change_bytes"));
        assert!(metrics.contains("# TYPE kopia_snapshot_size_change_bytes gauge"));
        assert!(
            metrics
                .contains("kopia_snapshot_size_change_bytes{source=\"user_name@host:/path\"} 1500"),
            "{metrics:?}"
        );
    }

    #[test]
    fn snapshot_size_change_negative() {
        let (map, _source) = single_map(vec![
            test_snapshot("1", 5000, &["daily-2"]),
            test_snapshot("2", 2000, &["latest-1"]),
        ]);

        let metrics = map
            .snapshot_size_change_bytes()
            .expect("nonempty")
            .to_string();

        assert!(
            metrics.contains(
                "kopia_snapshot_size_change_bytes{source=\"user_name@host:/path\"} -3000"
            ),
            "{metrics:?}"
        );
    }

    #[test]
    fn snapshot_size_change_single_snapshot() {
        let (map, _source) = single_map(vec![test_snapshot("1", 1000, &["latest-1"])]);

        let metrics = map.snapshot_size_change_bytes();
        assert!(metrics.is_none());
    }

    #[test]
    fn snapshot_size_change_empty() {
        let (map, _source) = single_map(vec![]);

        let metrics = map.snapshot_size_change_bytes();
        assert!(metrics.is_none());
    }
}
