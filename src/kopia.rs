use eyre::{Result, eyre};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

pub use self::source_map::SourceMap;
pub use self::source_str::{Error as SourceStrError, SourceStr};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[expect(missing_docs)] // no need to document all fields
pub struct Snapshot {
    pub id: String,
    pub source: Source,
    pub description: String,
    pub start_time: String,
    pub end_time: String,
    pub stats: Stats,
    pub root_entry: RootEntry,
    pub retention_reason: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[expect(missing_docs)] // no need to document all fields
pub struct Source {
    pub host: String,
    pub user_name: String,
    pub path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[expect(missing_docs)] // no need to document all fields
pub struct Stats {
    pub total_size: u64,
    pub excluded_total_size: u64,
    pub file_count: u32,
    pub cached_files: u32,
    pub non_cached_files: u32,
    pub dir_count: u32,
    pub excluded_file_count: u32,
    pub excluded_dir_count: u32,
    pub ignored_error_count: u32,
    pub error_count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[expect(missing_docs)] // no need to document all fields
pub struct RootEntry {
    pub name: String,
    #[serde(rename = "type")]
    pub entry_type: String,
    pub mode: String,
    pub mtime: String,
    pub obj: String,
    pub summ: Summary,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[expect(missing_docs)] // no need to document all fields
pub struct Summary {
    pub size: u64,
    pub files: u32,
    pub symlinks: u32,
    pub dirs: u32,
    pub max_time: String,
    pub num_failed: u32,
}

/// Parses JSON content into a vector of snapshots.
///
/// # Errors
///
/// Returns an error if the JSON content cannot be parsed as snapshot data, or
/// `invalid_source_fn` returns an error
pub(crate) fn parse_snapshots(
    json_content: &str,
    invalid_source_fn: impl Fn(source_str::Error) -> eyre::Result<()>,
) -> Result<SourceMap<Vec<Snapshot>>> {
    let snapshots: Vec<Snapshot> = serde_json::from_str(json_content)?;

    // organize by [`SourceStr`]
    let mut map = SourceMap::new();
    for snapshot in snapshots {
        let source_str = match snapshot.source.render() {
            Ok(s) => s,
            Err(e) => {
                invalid_source_fn(e)?;
                continue;
            }
        };
        let list: &mut Vec<Snapshot> = map.entry(source_str).or_default();
        list.push(snapshot);
    }
    Ok(map)
}

#[must_use]
pub fn get_retention_counts(
    snapshots_map: &SourceMap<Vec<Snapshot>>,
) -> SourceMap<BTreeMap<String, u32>> {
    snapshots_map
        .iter()
        .map(|(source, snapshots)| {
            let mut reason_counts = BTreeMap::<String, u32>::new();
            for snapshot in snapshots {
                for reason in &snapshot.retention_reason {
                    *reason_counts.entry(reason.clone()).or_insert(0) += 1;
                }
            }
            (source.clone(), reason_counts)
        })
        .collect()
}

/// Executes kopia command to retrieve snapshots and parses the output.
///
/// # Errors
///
/// Returns an error if:
/// - The kopia command fails to execute
/// - The command returns a non-zero exit code
/// - The command execution exceeds the specified timeout
/// - The output cannot be parsed as UTF-8
/// - The JSON output cannot be parsed as snapshot data
/// - `invalid_source_fn` returns an error
pub fn get_snapshots_from_command(
    kopia_bin: &str,
    timeout: Duration,
    invalid_source_fn: impl Fn(source_str::Error) -> eyre::Result<()>,
) -> Result<SourceMap<Vec<Snapshot>>> {
    let mut child = Command::new(kopia_bin)
        .args(["snapshot", "list", "--json"])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;

    let start = Instant::now();
    let poll_interval = Duration::from_millis(50);

    // Poll the child process until it completes or timeout is reached
    loop {
        if let Some(status) = child.try_wait()? {
            // Process completed
            let output = child.wait_with_output()?;

            if !status.success() {
                let stdout = String::from_utf8_lossy(&output.stdout);
                let stderr = String::from_utf8_lossy(&output.stderr);
                return Err(eyre!(
                    "kopia command failed with exit code: {}\nstdout: {}\nstderr: {}",
                    status.code().unwrap_or(-1),
                    stdout,
                    stderr
                ));
            }

            let stdout = String::from_utf8(output.stdout)?;
            let snapshots_map = parse_snapshots(&stdout, invalid_source_fn)?;
            return Ok(snapshots_map);
        }
        // Process still running, check timeout
        if start.elapsed() >= timeout {
            // Timeout exceeded, kill the process
            let _ = child.kill();
            let _ = child.wait();
            return Err(eyre!(
                "kopia command timeout after {} seconds",
                timeout.as_secs_f64()
            ));
        }
        // Sleep briefly before checking again
        thread::sleep(poll_interval);
    }
}

mod source_map {
    use crate::SourceStr;
    use std::collections::BTreeMap;

    /// Map from [`SourceStr`] to the desired data elements
    #[derive(Clone, Debug, Default)]
    pub struct SourceMap<T>(BTreeMap<SourceStr, T>);
    impl<T> SourceMap<T> {
        #[must_use]
        pub fn new() -> Self {
            Self(BTreeMap::new())
        }
        pub fn entry(
            &mut self,
            key: SourceStr,
        ) -> std::collections::btree_map::Entry<'_, SourceStr, T> {
            let Self(inner) = self;
            inner.entry(key)
        }
        /// Returns a single value if it is the only value
        ///
        /// # Errors
        /// Returns an error if the source is not found or is not the only value
        #[allow(clippy::missing_panics_doc)] // panic checks for logic error
        pub fn into_expect_only(mut self, source: &SourceStr) -> Result<T, Self> {
            let Self(inner) = &mut self;

            let 1 = inner.len() else {
                return Err(self);
            };

            let Some(value) = inner.remove(source) else {
                return Err(self);
            };

            assert!(inner.is_empty(), "length 1, removed 1, should be empty");
            Ok(value)
        }
        pub fn iter(&self) -> std::collections::btree_map::Iter<'_, SourceStr, T> {
            let Self(inner) = self;
            inner.iter()
        }
        #[must_use]
        pub fn is_empty(&self) -> bool {
            let Self(inner) = self;
            inner.is_empty()
        }
        pub fn map_nonempty<U>(self, map_fn: impl FnOnce(Self) -> U) -> Option<U> {
            if self.is_empty() {
                None
            } else {
                Some(map_fn(self))
            }
        }
    }
    impl<T> IntoIterator for SourceMap<T> {
        type Item = <BTreeMap<SourceStr, T> as IntoIterator>::Item;
        type IntoIter = <BTreeMap<SourceStr, T> as IntoIterator>::IntoIter;

        fn into_iter(self) -> Self::IntoIter {
            let Self(inner) = self;
            inner.into_iter()
        }
    }
    impl<'a, T> IntoIterator for &'a SourceMap<T> {
        type Item = (&'a SourceStr, &'a T);
        type IntoIter = std::collections::btree_map::Iter<'a, SourceStr, T>;
        fn into_iter(self) -> Self::IntoIter {
            self.0.iter()
        }
    }
    impl<T> FromIterator<(SourceStr, T)> for SourceMap<T> {
        fn from_iter<U: IntoIterator<Item = (SourceStr, T)>>(iter: U) -> Self {
            Self(iter.into_iter().collect())
        }
    }
}

mod source_str {
    use crate::Source;

    impl Source {
        /// Converts from the JSON/typed [`Source`] to a flat string [`SourceStr`]
        ///
        /// # Errors
        /// Returns an error if the `user_name` or `host` contain invalid characters that would
        /// make the flat string representation ambiguous
        pub fn render(&self) -> Result<SourceStr, Error> {
            let Self {
                host,
                user_name,
                path,
            } = self;

            let make_err = |kind| {
                Err(Error {
                    kind,
                    value_source: self.clone(),
                })
            };

            // reject invalid characters, to perserve uniqueness for SourceStr representation
            {
                let invalid_char = '@';
                if user_name.contains(invalid_char) {
                    return make_err(ErrorKind::InvalidUserName {
                        user_name: user_name.clone(),
                        invalid_char,
                    });
                }
            }
            {
                let invalid_char = ':';
                if host.contains(invalid_char) {
                    return make_err(ErrorKind::InvalidHost {
                        host: host.clone(),
                        invalid_char,
                    });
                }
            }

            let rendered = format!("{user_name}@{host}:{path}");
            Ok(SourceStr(rendered))
        }
    }
    /// String version for a [`Source`] rendered for output
    #[derive(Clone, Hash, PartialEq, Eq, PartialOrd, Ord)]
    pub struct SourceStr(String);
    impl SourceStr {
        #[must_use]
        pub fn new(value: String) -> Self {
            Self(value)
        }
    }
    impl std::fmt::Debug for SourceStr {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            let Self(text) = self;
            // wrap in Debug, to escape quotes
            write!(f, "{text:?}")
        }
    }

    #[derive(Debug)]
    pub struct Error {
        kind: ErrorKind,
        value_source: Source,
    }
    #[derive(Debug)]
    enum ErrorKind {
        InvalidUserName {
            user_name: String,
            invalid_char: char,
        },
        InvalidHost {
            host: String,
            invalid_char: char,
        },
    }
    impl std::error::Error for Error {
        fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
            match self.kind {
                ErrorKind::InvalidUserName { .. } | ErrorKind::InvalidHost { .. } => None,
            }
        }
    }
    impl std::fmt::Display for Error {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            let Self { kind, value_source } = self;
            match kind {
                ErrorKind::InvalidUserName {
                    user_name,
                    invalid_char,
                } => {
                    write!(
                        f,
                        "invalid char {invalid_char:?} in user name {user_name:?}"
                    )
                }
                ErrorKind::InvalidHost { host, invalid_char } => {
                    write!(f, "invalid char {invalid_char:?} in host {host:?}")
                }
            }?;
            write!(f, " in {value_source:?}")
        }
    }
}

#[cfg(test)]
pub(crate) mod test_util {
    #![allow(clippy::panic)] // tests can panic

    use super::*;

    #[track_caller]
    pub fn source_str(s: &str) -> SourceStr {
        SourceStr::new(s.to_string())
    }

    pub fn create_test_source(path: &str) -> Source {
        Source {
            host: "test".to_string(),
            user_name: "user".to_string(),
            path: path.to_string(),
        }
    }

    pub fn single_map(snapshots: Vec<Snapshot>) -> (SourceMap<Vec<Snapshot>>, SourceStr) {
        let source = Source {
            host: "host".to_string(),
            user_name: "user_name".to_string(),
            path: "/path".to_string(),
        }
        .render()
        .expect("valid source");

        let mut map = SourceMap::new();
        map.entry(source.clone()).insert_entry(snapshots);
        (map, source)
    }
}

#[cfg(test)]
pub mod tests {
    use crate::{
        RootEntry, Snapshot, Stats, Summary, get_retention_counts, parse_snapshots,
        test_util::{create_test_source, single_map, source_str},
    };

    fn create_test_snapshot(id: &str, total_size: u64, retention_reasons: &[&str]) -> Snapshot {
        Snapshot {
            id: id.to_string(),
            source: create_test_source("/test"),
            description: "".to_string(),
            start_time: "2025-08-14T00:00:00Z".to_string(),
            end_time: "2025-08-14T00:01:00Z".to_string(),
            stats: Stats {
                total_size,
                excluded_total_size: 0,
                file_count: 10,
                cached_files: 5,
                non_cached_files: 5,
                dir_count: 2,
                excluded_file_count: 0,
                excluded_dir_count: 0,
                ignored_error_count: 0,
                error_count: 0,
            },
            root_entry: RootEntry {
                name: "test".to_string(),
                entry_type: "d".to_string(),
                mode: "0755".to_string(),
                mtime: "2025-08-14T00:00:00Z".to_string(),
                obj: format!("obj{id}"),
                summ: Summary {
                    size: total_size,
                    files: 10,
                    symlinks: 0,
                    dirs: 2,
                    max_time: "2025-08-14T00:00:00Z".to_string(),
                    num_failed: 0,
                },
            },
            retention_reason: retention_reasons.iter().map(ToString::to_string).collect(),
        }
    }

    #[test]
    fn parse_single_snapshot() {
        let json = r#"[
            {
                "id": "test123",
                "source": {"host": "test", "userName": "user", "path": "/test"},
                "description": "",
                "startTime": "2025-08-14T00:00:00Z",
                "endTime": "2025-08-14T00:01:00Z",
                "stats": {
                    "totalSize": 1000,
                    "excludedTotalSize": 0,
                    "fileCount": 10,
                    "cachedFiles": 5,
                    "nonCachedFiles": 5,
                    "dirCount": 2,
                    "excludedFileCount": 0,
                    "excludedDirCount": 0,
                    "ignoredErrorCount": 0,
                    "errorCount": 0
                },
                "rootEntry": {
                    "name": "test",
                    "type": "d",
                    "mode": "0755",
                    "mtime": "2025-08-14T00:00:00Z",
                    "obj": "obj123",
                    "summ": {
                        "size": 1000,
                        "files": 10,
                        "symlinks": 0,
                        "dirs": 2,
                        "maxTime": "2025-08-14T00:00:00Z",
                        "numFailed": 0
                    }
                },
                "retentionReason": ["latest-1", "daily-1"]
            }
        ]"#;

        let snapshots = parse_snapshots(json, |e| eyre::bail!(e))
            .expect("valid JSON")
            .into_expect_only(&source_str("user@test:/test"))
            .expect("single source");
        assert_eq!(snapshots.len(), 1);
        assert_eq!(snapshots[0].id, "test123");
        assert_eq!(snapshots[0].stats.total_size, 1000);
        assert_eq!(snapshots[0].retention_reason, vec!["latest-1", "daily-1"]);
    }

    #[test]
    fn retention_counts_with_multiple_slots() {
        // This demonstrates that monthly-1, monthly-2, etc. should be counted separately
        // because they represent different retention slots, not multiple instances
        let (map, source) = single_map(vec![
            create_test_snapshot("snap1", 1000, &["latest-1", "daily-1", "monthly-1"]),
            create_test_snapshot("snap2", 2000, &["latest-2", "daily-2", "monthly-2"]),
        ]);

        let counts = get_retention_counts(&map)
            .into_expect_only(&source)
            .expect("single");

        // Each retention slot should be counted separately because they represent
        // different positions in the retention timeline, not duplicate instances
        assert_eq!(counts.get("latest-1"), Some(&1));
        assert_eq!(counts.get("latest-2"), Some(&1));
        assert_eq!(counts.get("daily-1"), Some(&1));
        assert_eq!(counts.get("daily-2"), Some(&1));
        assert_eq!(counts.get("monthly-1"), Some(&1));
        assert_eq!(counts.get("monthly-2"), Some(&1));

        // Verify we have the expected total number of distinct retention reasons
        assert_eq!(counts.len(), 6);

        // Verify no retention slot appears more than once (which would indicate
        // a problem with Kopia's retention policy or our test data)
        for count in counts.values() {
            assert_eq!(*count, 1);
        }
    }

    #[test]
    fn retention_counts() {
        let (map, source) = single_map(vec![
            create_test_snapshot("1", 1000, &["latest-1", "daily-1"]),
            create_test_snapshot("2", 2000, &["daily-2"]),
        ]);

        let counts = get_retention_counts(&map)
            .into_expect_only(&source)
            .expect("single");
        assert_eq!(counts.get("latest-1"), Some(&1));
        assert_eq!(counts.get("daily-1"), Some(&1));
        assert_eq!(counts.get("daily-2"), Some(&1));
    }

    #[test]
    fn parse_sample_data() {
        let sample_data = include_str!("sample_kopia-snapshot-list.json");
        let source = source_str("kopia-system@milton:/persist-home");

        let map = parse_snapshots(sample_data, |e| eyre::bail!(e)).expect("valid snapshot JSON");

        {
            // inspect parsed snapshots (for single source)
            let snapshots = map.clone().into_expect_only(&source).expect("single");

            assert_eq!(snapshots.len(), 17);

            let latest = snapshots.last().expect("nonempty");
            assert_eq!(latest.id, "c5be996d125abae92340f3a658443b24");
            assert_eq!(latest.start_time, "2025-08-14T00:00:04.04475167Z");
            assert_eq!(latest.stats.total_size, 42_154_950_324);
            assert_eq!(latest.stats.error_count, 0);
            assert_eq!(latest.root_entry.summ.num_failed, 0);
        }

        let retention_counts = get_retention_counts(&map);
        let retention_counts = retention_counts.into_expect_only(&source).expect("single");
        assert_eq!(retention_counts.get("latest-1"), Some(&1));
        assert_eq!(retention_counts.get("daily-1"), Some(&1));
        assert_eq!(retention_counts.get("monthly-1"), Some(&1));
    }
}
