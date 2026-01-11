use crate::{KopiaSnapshots, metrics::MetricLabel};
use std::fmt::{self, Display};

impl KopiaSnapshots {
    /// Generates Prometheus metrics for source parsing errors.
    ///
    /// Returns a string containing Prometheus-formatted metrics showing the count
    /// of snapshots with unparseable sources (invalid usernames or hostnames).
    /// Only present if there are parsing errors.
    #[must_use]
    pub(super) fn snapshot_source_parse_errors(&self) -> Option<impl Display> {
        const NAME: &str = "kopia_snapshot_source_parse_errors";
        const LABEL: MetricLabel =
            MetricLabel::gauge(NAME, "Number of snapshots with unparseable sources");

        struct Output<'a> {
            invalid_user_names: &'a std::collections::BTreeMap<String, u32>,
            invalid_hosts: &'a std::collections::BTreeMap<String, u32>,
        }
        impl Display for Output<'_> {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                let Self {
                    invalid_user_names,
                    invalid_hosts,
                } = self;

                writeln!(f, "{LABEL}")?;

                for (invalid_user, count) in *invalid_user_names {
                    writeln!(f, "{NAME}{{invalid_user={invalid_user:?}}} {count}")?;
                }

                for (invalid_host, count) in *invalid_hosts {
                    writeln!(f, "{NAME}{{invalid_host={invalid_host:?}}} {count}")?;
                }

                Ok(())
            }
        }

        if self.invalid_user_names.is_empty() && self.invalid_hosts.is_empty() {
            None
        } else {
            Some(Output {
                invalid_user_names: &self.invalid_user_names,
                invalid_hosts: &self.invalid_hosts,
            })
        }
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
        map.snapshot_source_parse_errors()
            .expect("has errors")
            .assert_contains_snippets(&["# HELP kopia_snapshot_source_parse_errors"])
            .assert_contains_lines(&[
                "# TYPE kopia_snapshot_source_parse_errors gauge",
                "kopia_snapshot_source_parse_errors{invalid_user=\"bad@user\"} 2",
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
        map.snapshot_source_parse_errors()
            .expect("has errors")
            .assert_contains_lines(&[
                "kopia_snapshot_source_parse_errors{invalid_host=\"bad:host\"} 1",
            ]);
    }

    #[test]
    fn source_parse_errors_none() {
        let snap = test_snapshot("1", 1000, &["latest-1"]);

        let map = KopiaSnapshots::new_from_snapshots(vec![snap], |_| Ok(())).expect("valid");
        let metrics = map.snapshot_source_parse_errors();

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
        map.snapshot_source_parse_errors()
            .expect("has errors")
            .assert_contains_lines(&[
                "kopia_snapshot_source_parse_errors{invalid_user=\"user@1\"} 1",
                "kopia_snapshot_source_parse_errors{invalid_host=\"host:2\"} 1",
            ]);
    }
}
