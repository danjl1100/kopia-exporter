use crate::{KopiaSnapshots, SourceMap, metrics::DisplayMetric};
use std::fmt;

pub(super) struct ParseErrorCountsTimestamp(SourceMap<u32>);
impl DisplayMetric for ParseErrorCountsTimestamp {
    fn fmt(&self, name: &str, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Self(error_counts) = self;
        for (source, error_count) in error_counts {
            writeln!(f, "{name}{{source={source:?}}} {error_count}")?;
        }
        Ok(())
    }
}

impl ParseErrorCountsTimestamp {
    pub fn new(ks: &KopiaSnapshots) -> Option<Self> {
        let error_counts: SourceMap<u32> = ks
            .snapshots_map
            .iter()
            .filter_map(|(source, snapshots)| {
                let error_count = snapshots
                    .iter()
                    .map(|snapshot| if snapshot.end_time.is_none() { 1 } else { 0 })
                    .sum::<u32>();

                (error_count > 0).then(|| (source.clone(), error_count))
            })
            .collect();

        error_counts.map_nonempty(Self)
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        AssertContains as _,
        test_util::{multi_map, test_snapshot},
    };

    #[test]
    fn snapshot_parse_errors_timestamp_multi_source() {
        let mut snapshot1 = test_snapshot("1", 1000, &["latest-1"]);
        snapshot1.end_time = "invalid-time".to_string();

        let mut snapshot2 = test_snapshot("2", 2000, &["latest-1"]);
        snapshot2.end_time = "also-invalid".to_string();

        let mut snapshot3 = test_snapshot("3", 3000, &["latest-1"]);
        snapshot3.end_time = "still-invalid".to_string();

        let (map, _sources) = multi_map(vec![
            ("alice", "hostA", "/data", vec![snapshot1, snapshot2]),
            ("bob", "hostB", "/backup", vec![snapshot3]),
        ]);

        map.kopia_snapshot_parse_errors_timestamp_total()
            .expect("nonempty")
            .assert_contains_snippets(&["# HELP kopia_snapshot_parse_errors_timestamp_total"])
            .assert_contains_lines(&[
                "# TYPE kopia_snapshot_parse_errors_timestamp_total gauge",
                "kopia_snapshot_parse_errors_timestamp_total{source=\"alice@hostA:/data\"} 2",
                "kopia_snapshot_parse_errors_timestamp_total{source=\"bob@hostB:/backup\"} 1",
            ]);
    }
}
