use crate::{
    KopiaSnapshots,
    metrics::{MetricLabel, last_snapshots::MetricLastSnapshots},
};
use std::fmt::Display;

impl KopiaSnapshots {
    /// Generates Prometheus metrics for ignored errors in the latest snapshot.
    ///
    /// Returns a string containing Prometheus-formatted metrics showing the total
    /// number of ignored errors in the most recent snapshot. Only present if snapshots list is not empty.
    #[must_use]
    pub(super) fn snapshot_ignored_errors_total(&self) -> Option<impl Display> {
        const NAME: &str = "kopia_snapshot_ignored_errors_total";
        const LABEL: MetricLabel = MetricLabel::gauge(NAME, "Ignored errors in latest snapshot");

        MetricLastSnapshots::new(self, NAME, LABEL, |v| v.stats.ignored_error_count)
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
