use crate::{KopiaSnapshots, SourceMap, metrics::DisplayMetric};
use std::fmt::{self};

pub(super) struct SnapshotAgeSeconds(SourceMap<i64>);
impl DisplayMetric for SnapshotAgeSeconds {
    fn fmt(&self, name: &str, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Self(age_seconds_map) = self;
        for (source, age_seconds) in age_seconds_map {
            writeln!(f, "{name}{{source={source:?}}} {age_seconds}")?;
        }

        Ok(())
    }
}
impl SnapshotAgeSeconds {
    /// Implementation for [`KopiaSnapshots::kopia_snapshot_age_seconds`]
    pub fn new(ks: &KopiaSnapshots, now: jiff::Timestamp) -> Option<Self> {
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
        age_seconds_map.map_nonempty(Self)
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

            map.kopia_snapshot_age_seconds(now)
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
        let metrics = map.kopia_snapshot_age_seconds(now);

        assert!(metrics.is_none());
    }

    #[test]
    fn snapshot_age_metric_invalid_time() {
        let mut snapshot = test_snapshot("1", 1000, &["latest-1"]);
        snapshot.end_time = "invalid-time".to_string();

        let now = jiff::Timestamp::now();

        let (map, _source) = single_map(vec![snapshot]);

        let age_metrics = map.kopia_snapshot_age_seconds(now);
        assert!(age_metrics.is_none());

        map.kopia_snapshot_parse_errors_timestamp_total()
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

        map.kopia_snapshot_age_seconds(now)
            .expect("nonempty")
            .assert_contains_snippets(&["# HELP kopia_snapshot_age_seconds"])
            .assert_contains_lines(&[
                "# TYPE kopia_snapshot_age_seconds gauge",
                "kopia_snapshot_age_seconds{source=\"alice@hostA:/data\"} 2700",
                "kopia_snapshot_age_seconds{source=\"bob@hostB:/backup\"} 7200",
            ]);
    }
}
