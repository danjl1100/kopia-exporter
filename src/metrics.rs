//! Defines metrics attached to [`KopiaSnapshots`]

use crate::KopiaSnapshots;
use std::fmt::{self, Display};

// Helpers
mod last_snapshots;

// Metric definitions
mod change_bytes;
mod error_totals;
mod failed_files;
mod ignored_errors;
mod last_age;
mod last_timestamp;
mod retention;
mod snapshots_total;
mod source_parse_errors;
mod total_size_bytes;

struct MetricLabel {
    name: &'static str,
    help_text: &'static str,
    ty: MetricType,
}
enum MetricType {
    Gauge,
}
impl MetricLabel {
    const fn gauge(name: &'static str, help_text: &'static str) -> Self {
        Self {
            name,
            help_text,
            ty: MetricType::Gauge,
        }
    }
}
impl Display for MetricLabel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Self {
            name,
            help_text,
            ty,
        } = self;
        let ty = match ty {
            MetricType::Gauge => "gauge",
        };

        write!(f, "# HELP {name} {help_text}")?;
        writeln!(f)?;
        write!(f, "# TYPE {name} {ty}")?;

        Ok(())
    }
}

impl KopiaSnapshots {
    /// Generates all Prometheus metrics for the `/metrics` endpoint.
    ///
    /// Combines all available metrics into a single response suitable for
    /// Prometheus scraping.
    #[must_use]
    pub fn generate_all_metrics(&self, now: jiff::Timestamp) -> String {
        struct Accumulator(String);
        impl Accumulator {
            fn new() -> Self {
                Self(String::new())
            }
            fn push(mut self, metric: Option<impl Display>) -> Self {
                use std::fmt::Write as _;
                if let Some(m) = metric {
                    let Self(output) = &mut self;
                    if !output.is_empty() {
                        output.push('\n');
                    }
                    write!(output, "{m}").expect("infallible");
                }
                self
            }
            fn finish(self) -> String {
                let Self(output) = self;
                output
            }
        }

        Accumulator::new()
            .push(Some(self.snapshots_by_retention()))
            .push(self.snapshot_total_size_bytes())
            .push(self.snapshot_age_seconds(now))
            .push(self.snapshot_timestamp_parse_errors_total())
            .push(self.snapshot_source_parse_errors())
            .push(self.snapshot_last_success_timestamp())
            .push(self.snapshot_errors_total())
            .push(self.snapshot_ignored_errors_total())
            .push(self.snapshot_failed_files_total())
            .push(self.snapshot_size_change_bytes())
            .push(Some(self.snapshots_total()))
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        AssertContains as _, KopiaSnapshots,
        test_util::{single_map, test_snapshot},
    };

    #[test]
    fn generate_all_metrics() {
        let snapshots = vec![test_snapshot("1", 1000, &["daily-1"])];

        let now = jiff::Timestamp::now();

        let (map, _source) = single_map(snapshots);
        map.generate_all_metrics(now).assert_contains_lines(&[
            "# TYPE kopia_snapshots_by_retention gauge",
            "# TYPE kopia_snapshot_total_size_bytes gauge",
            "# TYPE kopia_snapshot_age_seconds gauge",
            "# TYPE kopia_snapshot_errors_total gauge",
            "# TYPE kopia_snapshot_failed_files_total gauge",
            "# TYPE kopia_snapshots_total gauge",
        ]);
    }

    #[test]
    fn full_snapshot() {
        let sample_data = include_str!("sample_kopia-snapshot-list.json");
        let snapshots = KopiaSnapshots::new_parse_json(sample_data, |e| eyre::bail!(e))
            .expect("valid snapshot JSON");

        let now: jiff::Timestamp = "2025-08-17T20:58:04.972143344Z"
            .parse()
            .expect("valid timestamp");

        insta::assert_snapshot!(
            snapshots.generate_all_metrics(now),
            @r#"
            # HELP kopia_snapshots_by_retention Number of snapshots by retention reason
            # TYPE kopia_snapshots_by_retention gauge
            kopia_snapshots_by_retention{source="kopia-system@milton:/persist-home",retention_reason="annual-1"} 1
            kopia_snapshots_by_retention{source="kopia-system@milton:/persist-home",retention_reason="daily-1"} 1
            kopia_snapshots_by_retention{source="kopia-system@milton:/persist-home",retention_reason="daily-2"} 1
            kopia_snapshots_by_retention{source="kopia-system@milton:/persist-home",retention_reason="daily-3"} 1
            kopia_snapshots_by_retention{source="kopia-system@milton:/persist-home",retention_reason="daily-4"} 1
            kopia_snapshots_by_retention{source="kopia-system@milton:/persist-home",retention_reason="daily-5"} 1
            kopia_snapshots_by_retention{source="kopia-system@milton:/persist-home",retention_reason="daily-6"} 1
            kopia_snapshots_by_retention{source="kopia-system@milton:/persist-home",retention_reason="hourly-1"} 1
            kopia_snapshots_by_retention{source="kopia-system@milton:/persist-home",retention_reason="hourly-2"} 1
            kopia_snapshots_by_retention{source="kopia-system@milton:/persist-home",retention_reason="hourly-3"} 1
            kopia_snapshots_by_retention{source="kopia-system@milton:/persist-home",retention_reason="hourly-4"} 1
            kopia_snapshots_by_retention{source="kopia-system@milton:/persist-home",retention_reason="hourly-5"} 1
            kopia_snapshots_by_retention{source="kopia-system@milton:/persist-home",retention_reason="latest-1"} 1
            kopia_snapshots_by_retention{source="kopia-system@milton:/persist-home",retention_reason="latest-10"} 1
            kopia_snapshots_by_retention{source="kopia-system@milton:/persist-home",retention_reason="latest-2"} 1
            kopia_snapshots_by_retention{source="kopia-system@milton:/persist-home",retention_reason="latest-3"} 1
            kopia_snapshots_by_retention{source="kopia-system@milton:/persist-home",retention_reason="latest-4"} 1
            kopia_snapshots_by_retention{source="kopia-system@milton:/persist-home",retention_reason="latest-5"} 1
            kopia_snapshots_by_retention{source="kopia-system@milton:/persist-home",retention_reason="latest-6"} 1
            kopia_snapshots_by_retention{source="kopia-system@milton:/persist-home",retention_reason="latest-7"} 1
            kopia_snapshots_by_retention{source="kopia-system@milton:/persist-home",retention_reason="latest-8"} 1
            kopia_snapshots_by_retention{source="kopia-system@milton:/persist-home",retention_reason="latest-9"} 1
            kopia_snapshots_by_retention{source="kopia-system@milton:/persist-home",retention_reason="monthly-1"} 1
            kopia_snapshots_by_retention{source="kopia-system@milton:/persist-home",retention_reason="monthly-2"} 1
            kopia_snapshots_by_retention{source="kopia-system@milton:/persist-home",retention_reason="monthly-3"} 1
            kopia_snapshots_by_retention{source="kopia-system@milton:/persist-home",retention_reason="monthly-4"} 1
            kopia_snapshots_by_retention{source="kopia-system@milton:/persist-home",retention_reason="weekly-1"} 1
            kopia_snapshots_by_retention{source="kopia-system@milton:/persist-home",retention_reason="weekly-2"} 1
            kopia_snapshots_by_retention{source="kopia-system@milton:/persist-home",retention_reason="weekly-3"} 1
            kopia_snapshots_by_retention{source="kopia-system@milton:/persist-home",retention_reason="weekly-4"} 1

            # HELP kopia_snapshot_total_size_bytes Total size of latest snapshot in bytes
            # TYPE kopia_snapshot_total_size_bytes gauge
            kopia_snapshot_total_size_bytes{source="kopia-system@milton:/persist-home"} 42154950324

            # HELP kopia_snapshot_age_seconds Age of newest snapshot in seconds
            # TYPE kopia_snapshot_age_seconds gauge
            kopia_snapshot_age_seconds{source="kopia-system@milton:/persist-home"} 334678

            # HELP kopia_snapshot_last_success_timestamp Unix timestamp of last successful snapshot
            # TYPE kopia_snapshot_last_success_timestamp gauge
            kopia_snapshot_last_success_timestamp{source="kopia-system@milton:/persist-home"} 1755129606

            # HELP kopia_snapshot_errors_total Total errors in latest snapshot
            # TYPE kopia_snapshot_errors_total gauge
            kopia_snapshot_errors_total{source="kopia-system@milton:/persist-home"} 0

            # HELP kopia_snapshot_ignored_errors_total Ignored errors in latest snapshot
            # TYPE kopia_snapshot_ignored_errors_total gauge
            kopia_snapshot_ignored_errors_total{source="kopia-system@milton:/persist-home"} 0

            # HELP kopia_snapshot_failed_files_total Number of failed files in latest snapshot
            # TYPE kopia_snapshot_failed_files_total gauge
            kopia_snapshot_failed_files_total{source="kopia-system@milton:/persist-home"} 0

            # HELP kopia_snapshot_size_change_bytes Change in size from previous snapshot
            # TYPE kopia_snapshot_size_change_bytes gauge
            kopia_snapshot_size_change_bytes{source="kopia-system@milton:/persist-home"} 1116951

            # HELP kopia_snapshots_total Total number of snapshots
            # TYPE kopia_snapshots_total gauge
            kopia_snapshots_total{source="kopia-system@milton:/persist-home"} 17
            "#
        );
    }
}
