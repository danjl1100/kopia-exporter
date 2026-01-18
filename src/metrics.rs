//! Defines metrics attached to [`KopiaSnapshots`]

use crate::KopiaSnapshots;
use std::fmt::{self, Display};

/// Label and data for a specific metric
///
/// See associated constants for a list of implemented metric types
pub struct Metrics<T> {
    label: MetricLabel,
    inner: T,
}
impl<T> std::fmt::Display for Metrics<T>
where
    T: DisplayMetric,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Self { label, inner } = self;

        // format label
        writeln!(f, "{label}")?;

        // format inner
        let name = label.name();
        inner.fmt(name, f)
    }
}
/// [`std::fmt::Display`], but with an additional supplied metric name
trait DisplayMetric {
    fn fmt(&self, name: &str, f: &mut fmt::Formatter<'_>) -> fmt::Result;
}

/// Helper to construct [`Metrics`] from various disjoint types
trait AttachMetricLabel {
    type Output;
    fn attach_metric_label(self, label: MetricLabel) -> Self::Output;
}
// NOTE: `(T,)` required to disambiguate with the blanket impl covering `T = Option<...>`
impl<T> AttachMetricLabel for (T,) {
    type Output = Metrics<T>;
    fn attach_metric_label(self, label: MetricLabel) -> Self::Output {
        let (inner,) = self;
        Metrics { label, inner }
    }
}
impl<T> AttachMetricLabel for Option<T> {
    type Output = Option<Metrics<T>>;
    fn attach_metric_label(self, label: MetricLabel) -> Self::Output {
        self.map(|inner| (inner,).attach_metric_label(label))
    }
}

macro_rules! define_metric_categories {
    (
        // Repeat - categories
        $(
            // Category help text, as a doc comment: `/// xxxxxxx`
            #[doc = $category:literal]
            // Category identifier
            $category_ident:ident
            :
            impl $Container:ident {
                // Repeat - metrics
                $(
                    // First line of doc text - used for the `# HELP` text
                    #[doc = $help:literal]
                    $(#[$meta:meta])*
                    $vis:vis fn $name:ident<$ty:ident>($($tt:tt)*) -> $return_ty:ty $block:block
                )+
            }
        )+
    ) => {
        $(
            // Define category (docs only) and metrics (docs and provide the MetricLabel)
            impl<T> Metrics<T> {
                /// **CATEGORY**:
                #[doc = $category]
                ///
                /// ---
                /// Individual metrics are listed in the group below
                pub const $category_ident: () = ();

                $(
                    #[doc = concat!("Metric: `", stringify!($name), "`")]
                    ///
                    #[doc = concat!("(", stringify!($ty), ")")]
                    #[doc = concat!($help)]
                    #[doc = concat!("([implementation](`", stringify!($Container), "::", stringify!($name), "`))")]
                    #[expect(non_upper_case_globals)]
                    pub const $name: $crate::metrics::MetricLabel =
                        $crate::metrics::MetricLabel::__from_macro(
                            stringify!($name),
                            $help.trim_ascii_start(),
                            $crate::metrics::MetricType::$ty,
                        );
                )+
            }

            // Import each metric implementation module, not exported
            //
            // Items in the implementation module are automatically imported
            // for use in each Container function, see below
            $(
                mod $name;
            )+

            // Define methods on $Container for each metric
            impl $Container {
                $(
                    #[doc = concat!("Metric `", stringify!($name), "` - ", $help)]
                    ///
                    #[doc = concat!("Category: [", $category, "](Metrics::", stringify!($category_ident), ")")]
                    ///
                    /// ---
                    ///
                    $(#[$meta])*
                    #[must_use]
                    $vis fn $name($($tt)*) -> $return_ty {
                        #[allow(unused_imports)]
                        use $name::*;

                        let inner = $block;
                        inner.attach_metric_label(
                            Metrics::<()>::$name,
                        )
                    }
                )+
            }
        )+
    };
}

define_metric_categories! {
    /// New snapshot health
    NEW_SNAPSHOT_HEALTH: impl KopiaSnapshots {
        /// Age of newest snapshot in seconds
        ///
        /// Returns metrics showing the age in seconds of the most recent snapshot for each source.
        /// Only present if snapshots list is not empty.
        pub fn kopia_snapshot_age_seconds<Gauge>(&self, now: jiff::Timestamp) -> Option<impl Display> {
            SnapshotAgeSeconds::new(self, now)
        }
        /// Unix timestamp of last successful snapshot
        ///
        /// Generates Prometheus metrics for the last successful snapshot timestamp.
        /// Only present if snapshots list is not empty.
        pub fn kopia_snapshot_last_success_timestamp<Gauge>(&self) -> Option<impl Display> {
            SnapshotLastSuccessTimestamp::new(self)
        }
    }
}
define_metric_categories! {
    /// Backup completion status
    BACKUP_COMPLETION_STATUS: impl KopiaSnapshots {
        /// Total errors in latest snapshot
        ///
        /// Returns metrics showing the total number of errors in the most recent snapshot.
        /// Only present if snapshots list is not empty.
        pub fn kopia_snapshot_errors_total<Gauge>(&self) -> Option<impl Display> {
            last_snapshots::MetricLastSnapshots::new(self, |v| v.stats.error_count)
        }
        /// Ignored errors in latest snapshot
        ///
        /// Returns a string containing Prometheus-formatted metrics showing the total
        /// number of ignored errors in the most recent snapshot. Only present if snapshots list is not empty.
        pub fn kopia_snapshot_errors_ignored_total<Gauge>(&self) -> Option<impl Display> {
            last_snapshots::MetricLastSnapshots::new(self, |v| v.stats.ignored_error_count)
        }
    }
}
define_metric_categories! {
    /// Data integrity verification
    DATA_INTEGRITY_VERIFICATION: impl KopiaSnapshots {
        /// Number of failed files in latest snapshot
        ///
        /// Returns metrics showing the number of failed files in the most recent snapshot.
        /// Only present if snapshots list is not empty.
        pub fn kopia_snapshot_failed_files_total<Gauge>(&self) -> Option<impl Display> {
            last_snapshots::MetricLastSnapshots::new(self, |v| v.root_entry.summ.num_failed)
        }
    }
}
define_metric_categories! {
    /// Remaining space
    REMAINING_SPACE: impl KopiaSnapshots {
        /// Total size of latest snapshot in bytes
        ///
        /// Returns metrics showing the total size in bytes of the most recent snapshot.
        /// Only present if snapshots list is not empty.
        pub fn kopia_snapshot_size_bytes_total<Gauge>(&self) -> Option<impl Display> {
            last_snapshots::MetricLastSnapshots::new(self, |v| v.stats.total_size)
        }
        /// Change in size from previous snapshot
        ///
        /// Returns metrics showing the change in bytes from the previous snapshot.
        /// Only present if snapshots list has more than one snapshot.
        pub fn kopia_snapshot_size_bytes_change<Gauge>(&self) -> Option<impl Display> {
            SnapshotSizeByteChanges::new(self)
        }
    }
}
define_metric_categories! {
    /// Pruned snapshots
    PRUNED_SNAPSHOTS: impl KopiaSnapshots {
        /// Number of snapshots by retention reason
        ///
        /// Returns metrics showing the count of snapshots for each retention reason
        /// (e.g., "latest-1", "daily-7", etc.).
        pub fn kopia_snapshots_by_retention<Gauge>(&self) -> impl Display {
            let always = SnapshotsByRetention::new(self);
            (always,)
        }
        /// Total number of snapshots
        ///
        /// Returns metrics showing the total count of all snapshots in the repository.
        pub fn kopia_snapshots_total<Gauge>(&self) -> impl Display {
            let always = SnapshotsTotal::new(self);
            (always,)
        }
    }
}
define_metric_categories! {
    /// Data quality
    DATA_QUALITY: impl KopiaSnapshots {
        /// Number of snapshots with unparseable sources
        ///
        /// Returns metrics showing the count of snapshots with unparseable sources
        /// (invalid usernames or hostnames).
        /// Only present if there are parsing errors.
        pub fn kopia_snapshot_parse_errors_source<Gauge>(&self) -> Option<impl Display> {
            SnapshotParseErrorsSource::new(self)
        }
        /// Number of snapshots with unparseable timestamps
        ///
        /// Returns metrics showing the count of snapshots with unparseable timestamps.
        /// Only present if there are parsing errors.
        pub fn kopia_snapshot_parse_errors_timestamp_total<Gauge>(&self) -> Option<impl Display> {
            ParseErrorCountsTimestamp::new(self)
        }
    }
}

// Helpers
mod last_snapshots;

/// Label (name, type, and help text) for a specific kind of metric
pub struct MetricLabel {
    name: &'static str,
    help_text: &'static str,
    ty: MetricType,
}
/// Type of a prometheus metric
///
/// See more details at the [Prometheus docs](https://prometheus.io/docs/concepts/metric_types/)
pub enum MetricType {
    /// Monotonically increasing value - can only increase or be reset to zero on restart
    Counter,
    /// Single numerical value that can arbitrarily go up and down
    Gauge,
}

impl MetricLabel {
    /// Internal constructor for use by the `define_metric!` macro.
    ///
    /// This method should not be called directly. Use the `define_metric!` macro instead.
    #[doc(hidden)]
    #[must_use]
    pub const fn __from_macro(name: &'static str, help_text: &'static str, ty: MetricType) -> Self {
        Self {
            name,
            help_text,
            ty,
        }
    }
    /// Returns the name of the metric
    #[must_use]
    pub fn name(&self) -> &str {
        self.name
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
            MetricType::Counter => "counter",
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
            .push(Some(self.kopia_snapshots_by_retention()))
            .push(self.kopia_snapshot_size_bytes_total())
            .push(self.kopia_snapshot_age_seconds(now))
            .push(self.kopia_snapshot_parse_errors_timestamp_total())
            .push(self.kopia_snapshot_parse_errors_source())
            .push(self.kopia_snapshot_last_success_timestamp())
            .push(self.kopia_snapshot_errors_total())
            .push(self.kopia_snapshot_errors_ignored_total())
            .push(self.kopia_snapshot_failed_files_total())
            .push(self.kopia_snapshot_size_bytes_change())
            .push(Some(self.kopia_snapshots_total()))
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
            "# TYPE kopia_snapshot_size_bytes_total gauge",
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

            # HELP kopia_snapshot_size_bytes_total Total size of latest snapshot in bytes
            # TYPE kopia_snapshot_size_bytes_total gauge
            kopia_snapshot_size_bytes_total{source="kopia-system@milton:/persist-home"} 42154950324

            # HELP kopia_snapshot_age_seconds Age of newest snapshot in seconds
            # TYPE kopia_snapshot_age_seconds gauge
            kopia_snapshot_age_seconds{source="kopia-system@milton:/persist-home"} 334678

            # HELP kopia_snapshot_last_success_timestamp Unix timestamp of last successful snapshot
            # TYPE kopia_snapshot_last_success_timestamp gauge
            kopia_snapshot_last_success_timestamp{source="kopia-system@milton:/persist-home"} 1755129606

            # HELP kopia_snapshot_errors_total Total errors in latest snapshot
            # TYPE kopia_snapshot_errors_total gauge
            kopia_snapshot_errors_total{source="kopia-system@milton:/persist-home"} 0

            # HELP kopia_snapshot_errors_ignored_total Ignored errors in latest snapshot
            # TYPE kopia_snapshot_errors_ignored_total gauge
            kopia_snapshot_errors_ignored_total{source="kopia-system@milton:/persist-home"} 0

            # HELP kopia_snapshot_failed_files_total Number of failed files in latest snapshot
            # TYPE kopia_snapshot_failed_files_total gauge
            kopia_snapshot_failed_files_total{source="kopia-system@milton:/persist-home"} 0

            # HELP kopia_snapshot_size_bytes_change Change in size from previous snapshot
            # TYPE kopia_snapshot_size_bytes_change gauge
            kopia_snapshot_size_bytes_change{source="kopia-system@milton:/persist-home"} 1116951

            # HELP kopia_snapshots_total Total number of snapshots
            # TYPE kopia_snapshots_total gauge
            kopia_snapshots_total{source="kopia-system@milton:/persist-home"} 17
            "#
        );
    }
}
