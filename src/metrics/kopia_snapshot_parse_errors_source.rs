use crate::{KopiaSnapshots, metrics::DisplayMetric};
use std::fmt;

pub(super) struct SnapshotParseErrorsSource<'a> {
    invalid_user_names: &'a std::collections::BTreeMap<String, u32>,
    invalid_hosts: &'a std::collections::BTreeMap<String, u32>,
}
impl<'a> SnapshotParseErrorsSource<'a> {
    pub fn new(ks: &'a KopiaSnapshots) -> Option<Self> {
        let KopiaSnapshots {
            invalid_user_names,
            invalid_hosts,
            ..
        } = ks;
        if invalid_user_names.is_empty() && invalid_hosts.is_empty() {
            None
        } else {
            Some(Self {
                invalid_user_names,
                invalid_hosts,
            })
        }
    }
}
impl DisplayMetric for SnapshotParseErrorsSource<'_> {
    fn fmt(&self, name: &str, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Self {
            invalid_user_names,
            invalid_hosts,
        } = self;

        for (invalid_user, count) in *invalid_user_names {
            writeln!(f, "{name}{{invalid_user={invalid_user:?}}} {count}")?;
        }

        for (invalid_host, count) in *invalid_hosts {
            writeln!(f, "{name}{{invalid_host={invalid_host:?}}} {count}")?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::{AssertContains as _, KopiaSnapshots, Source, test_util::test_snapshot};

    #[test]
    fn source_parse_errors_invalid_user() {
        let mut snap1 = test_snapshot("1", 1000, &["latest-1"]);
        snap1.source = Source {
            host: "myhost".to_string(),
            user_name: "bad@user".to_string(),
            path: "/test".to_string(),
        };

        let mut snap2 = test_snapshot("2", 1000, &["latest-1"]);
        snap2.source = Source {
            host: "myhost".to_string(),
            user_name: "bad@user".to_string(),
            path: "/test2".to_string(),
        };

        let map =
            KopiaSnapshots::new_from_snapshots(vec![snap1, snap2], |_| Ok(())).expect("valid");
        map.kopia_snapshot_parse_errors_source()
            .expect("has errors")
            .assert_contains_snippets(&["# HELP kopia_snapshot_parse_errors_source"])
            .assert_contains_lines(&[
                "# TYPE kopia_snapshot_parse_errors_source gauge",
                "kopia_snapshot_parse_errors_source{invalid_user=\"bad@user\"} 2",
            ]);
    }

    #[test]
    fn source_parse_errors_invalid_host() {
        let mut snap = test_snapshot("1", 1000, &["latest-1"]);
        snap.source = Source {
            host: "bad:host".to_string(),
            user_name: "user".to_string(),
            path: "/test".to_string(),
        };

        let map = KopiaSnapshots::new_from_snapshots(vec![snap], |_| Ok(())).expect("valid");
        map.kopia_snapshot_parse_errors_source()
            .expect("has errors")
            .assert_contains_lines(&[
                "kopia_snapshot_parse_errors_source{invalid_host=\"bad:host\"} 1",
            ]);
    }

    #[test]
    fn source_parse_errors_none() {
        let snap = test_snapshot("1", 1000, &["latest-1"]);

        let map = KopiaSnapshots::new_from_snapshots(vec![snap], |_| Ok(())).expect("valid");
        let metrics = map.kopia_snapshot_parse_errors_source();

        assert!(metrics.is_none());
    }

    #[test]
    fn source_parse_errors_multiple_different_values() {
        let mut snap1 = test_snapshot("1", 1000, &["latest-1"]);
        snap1.source = Source {
            host: "host1".to_string(),
            user_name: "user@1".to_string(),
            path: "/test".to_string(),
        };

        let mut snap2 = test_snapshot("2", 1000, &["latest-1"]);
        snap2.source = Source {
            host: "host:2".to_string(),
            user_name: "user2".to_string(),
            path: "/test".to_string(),
        };

        let map =
            KopiaSnapshots::new_from_snapshots(vec![snap1, snap2], |_| Ok(())).expect("valid");
        map.kopia_snapshot_parse_errors_source()
            .expect("has errors")
            .assert_contains_lines(&[
                "kopia_snapshot_parse_errors_source{invalid_user=\"user@1\"} 1",
                "kopia_snapshot_parse_errors_source{invalid_host=\"host:2\"} 1",
            ]);
    }
}
