//! Framework for displaying Prometheus-formatted metrics,
//! agnostic of the specific application being instrumented

use std::fmt;

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
impl fmt::Display for MetricLabel {
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

/// [`std::fmt::Display`], but with an additional supplied metric name
pub trait DisplayMetric {
    fn fmt(&self, name: &str, f: &mut fmt::Formatter<'_>) -> fmt::Result;
}

/// Helper to construct [`Metrics`] from various disjoint types
pub trait AttachMetricLabel {
    /// Output form of (possibly wrapped) [`Metrics`]
    type Output;
    /// Wraps `self` in [`Metrics`], as appropriate for the container
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

/// Defines categories for metrics, annotating as constants on the target metrics
///
/// # Examples
///
/// ```ignore
/// define_metric_categories! {
///     /// New snapshot health
///     NEW_SNAPSHOT_HEALTH: impl KopiaSnapshots {
///         /// Age of newest snapshot in seconds
///         ///
///         /// Returns metrics showing the age in seconds of the most recent snapshot for each source.
///         /// Only present if snapshots list is not empty.
///         pub fn kopia_snapshot_age_seconds<Gauge>(&self, now: jiff::Timestamp) -> Option<impl Display> {
///             SnapshotAgeSeconds::new(self, now)
///         }
///         /// Unix timestamp of last successful snapshot
///         ///
///         /// Generates Prometheus metrics for the last successful snapshot timestamp.
///         /// Only present if snapshots list is not empty.
///         pub fn kopia_snapshot_last_success_timestamp<Gauge>(&self) -> Option<impl Display> {
///             SnapshotLastSuccessTimestamp::new(self)
///         }
///     }
/// }
/// ```
#[macro_export]
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
