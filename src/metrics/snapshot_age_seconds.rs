//! **New snapshot health:** Age of newest snapshot in seconds

use crate::{KopiaSnapshots, SourceMap};
use std::fmt::{self, Display};

macro_rules! define_metric_new {
    (
        $(#[$meta:meta])*
        $vis:vis fn $name:ident($($tt:tt)*) -> $return:ty $block:block
    ) => {
        $(#[$meta])*
        $vis fn $name($($tt::tt)*) -> $return $block
    };
}

impl KopiaSnapshots {
    define_metric_new! {
        /// Generates Prometheus metrics for the age of the latest snapshot.
        ///
        /// Returns a string containing Prometheus-formatted metrics showing the age
        /// in seconds of the most recent snapshot from its end time. Only present if snapshots list is not empty.
        #[must_use]
        pub(super) fn snapshot_age_seconds(&self, now: jiff::Timestamp) -> Option<impl Display> {
            SnapshotAgeSeconds::new(self, now)
        }
    }
}

struct SnapshotAgeSeconds {
    age_seconds_map: SourceMap<i64>,
}
impl Display for SnapshotAgeSeconds {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Self { age_seconds_map } = self;
        writeln!(f, "{LABEL}")?;
        for (source, age_seconds) in age_seconds_map {
            writeln!(f, "{NAME}{{source={source:?}}} {age_seconds}")?;
        }

        Ok(())
    }
}
impl SnapshotAgeSeconds {
    fn new(ks: &KopiaSnapshots, now: jiff::Timestamp) -> Option<Self> {
        let age_seconds_map: SourceMap<_> = ks
            .snapshots_map
            .iter()
            .filter_map(|(source, snapshots)| {
                let last = snapshots.last()?;
                let age_seconds = {
                    let age = now - last.end_time?;
                    let age_seconds = age
                        .total(jiff::Unit::Second)
                        .expect("relative reference time given");
                    #[expect(clippy::cast_possible_truncation)]
                    {
                        age_seconds.round() as i64
                    }
                };
                Some((source.clone(), age_seconds))
            })
            .collect();
        if age_seconds_map.is_empty() {
            None
        } else {
            Some(Output { age_seconds_map })
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        AssertContains as _,
        test_util::{multi_map, single_map, test_snapshot},
    };

    #[test]
    fn snapshot_age_metrics() {
        use jiff::ToSpan as _;

        for minutes in [30, 100] {
            let now = jiff::Timestamp::now();
            let recent_time = now - minutes.minutes();
            let mut snapshot = test_snapshot("1", 1000, &["latest-1"]);
            snapshot.end_time = recent_time.to_string();

            let seconds = minutes * 60;

            let (map, _source) = single_map(vec![snapshot]);

            map.snapshot_age_seconds(now)
                .expect("nonempty")
                .assert_contains_snippets(&["# HELP kopia_snapshot_age_seconds"])
                .assert_contains_lines(&[
                    "# TYPE kopia_snapshot_age_seconds gauge",
                    &format!(
                        "kopia_snapshot_age_seconds{{source=\"user_name@host:/path\"}} {seconds}"
                    ),
                ]);
        }
    }

    #[test]
    fn snapshot_age_metrics_empty() {
        let (map, _source) = single_map(vec![]);
        let now = jiff::Timestamp::now();
        let metrics = map.snapshot_age_seconds(now);

        assert!(metrics.is_none());
    }

    #[test]
    fn snapshot_age_metric_invalid_time() {
        let mut snapshot = test_snapshot("1", 1000, &["latest-1"]);
        snapshot.end_time = "invalid-time".to_string();

        let now = jiff::Timestamp::now();

        let (map, _source) = single_map(vec![snapshot]);

        let age_metrics = map.snapshot_age_seconds(now);
        assert!(age_metrics.is_none());

        map.snapshot_parse_errors_timestamp_total()
            .expect("nonempty")
            .assert_contains_lines(&[
                "kopia_snapshot_parse_errors_timestamp_total{source=\"user_name@host:/path\"} 1",
            ]);
    }

    #[test]
    fn snapshot_age_multi_source() {
        use jiff::ToSpan as _;

        let now = jiff::Timestamp::now();
        let age1 = 45.minutes();
        let age2 = 120.minutes();

        let mut snapshot1 = test_snapshot("1", 1000, &["latest-1"]);
        snapshot1.end_time = (now - age1).to_string();

        let mut snapshot2 = test_snapshot("2", 2000, &["latest-1"]);
        snapshot2.end_time = (now - age2).to_string();

        let (map, _sources) = multi_map(vec![
            ("alice", "hostA", "/data", vec![snapshot1]),
            ("bob", "hostB", "/backup", vec![snapshot2]),
        ]);

        map.snapshot_age_seconds(now)
            .expect("nonempty")
            .assert_contains_snippets(&["# HELP kopia_snapshot_age_seconds"])
            .assert_contains_lines(&[
                "# TYPE kopia_snapshot_age_seconds gauge",
                "kopia_snapshot_age_seconds{source=\"alice@hostA:/data\"} 2700",
                "kopia_snapshot_age_seconds{source=\"bob@hostB:/backup\"} 7200",
            ]);
    }
}
