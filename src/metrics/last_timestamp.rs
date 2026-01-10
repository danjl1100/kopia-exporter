use crate::{KopiaSnapshots, SourceMap, metrics::MetricLabel};
use std::fmt::{self, Display};

impl KopiaSnapshots {
    /// Generates Prometheus metrics for the last successful snapshot timestamp.
    ///
    /// Returns a string containing Prometheus-formatted metrics showing the Unix timestamp
    /// of the most recent snapshot. Only present if snapshots list is not empty.
    #[must_use]
    pub(super) fn snapshot_last_success_timestamp(&self) -> Option<impl Display> {
        const NAME: &str = "kopia_snapshot_last_success_timestamp";
        const LABEL: MetricLabel =
            MetricLabel::gauge(NAME, "Unix timestamp of last successful snapshot");

        struct Timestamps(SourceMap<i64>);
        impl Display for Timestamps {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                let Self(timestamps) = self;
                writeln!(f, "{LABEL}")?;
                for (source, timestamp) in timestamps {
                    writeln!(f, "{NAME}{{source={source:?}}} {timestamp}")?;
                }
                Ok(())
            }
        }

        let timestamps: SourceMap<i64> = self
            .snapshots_map
            .iter()
            .filter_map(|(source, snapshots)| {
                let last = snapshots.last()?;
                let end_time = last.end_time?;
                Some((source.clone(), end_time.as_second()))
            })
            .collect();

        timestamps.map_nonempty(Timestamps)
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
