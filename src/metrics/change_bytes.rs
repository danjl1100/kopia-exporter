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
    #[test]
    #[ignore = "TODO"]
    fn todo() {
        todo!()
    }
}
