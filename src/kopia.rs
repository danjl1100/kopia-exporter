//! Parsed types for `kopia` snapshot listings in JSON format

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub use self::source_map::SourceMap;
pub use self::source_str::{Error as SourceStrError, SourceStr};
use crate::KopiaSnapshots;

mod source_map;
mod source_str;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[expect(missing_docs)] // no need to document all fields
pub struct SnapshotJson {
    pub id: String,
    pub source: Source,
    pub description: String,
    pub start_time: String,
    pub end_time: String,
    pub stats: Stats,
    pub root_entry: RootEntry,
    pub retention_reason: Vec<String>,
}

#[derive(Debug, Clone)]
#[expect(missing_docs)] // no need to document all fields
pub struct Snapshot {
    pub id: String,
    pub source: Source,
    pub description: String,
    pub start_time: String,
    pub end_time: Option<jiff::Timestamp>,
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

impl From<SnapshotJson> for Snapshot {
    fn from(value: SnapshotJson) -> Self {
        let SnapshotJson {
            id,
            source,
            description,
            start_time,
            end_time,
            stats,
            root_entry,
            retention_reason,
        } = value;
        Self {
            id,
            source,
            description,
            start_time,
            end_time: end_time.parse().ok(),
            stats,
            root_entry,
            retention_reason,
        }
    }
}

impl KopiaSnapshots {
    /// Returns the number of snapshots for each [`Snapshot::retention_reason`]
    #[must_use]
    pub fn get_retention_counts(&self) -> SourceMap<BTreeMap<String, u32>> {
        self.snapshots_map
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
}

#[cfg(test)]
pub(crate) mod test_util {
    use super::*;

    #[track_caller]
    pub fn source_str(s: &str) -> SourceStr {
        SourceStr::new_unchecked(s.to_string())
    }

    pub fn single_map(snapshots: Vec<SnapshotJson>) -> (KopiaSnapshots, SourceStr) {
        let source = Source {
            host: "host".to_string(),
            user_name: "user_name".to_string(),
            path: "/path".to_string(),
        }
        .render()
        .expect("valid source");

        let map =
            KopiaSnapshots::new_from_snapshots(snapshots, |_| Ok(())).expect("valid snapshots");

        (map, source)
    }

    pub fn test_snapshot(id: &str, total_size: u64, retention_reasons: &[&str]) -> SnapshotJson {
        test_snapshot_with_source(
            id,
            total_size,
            retention_reasons,
            Source {
                host: "host".to_string(),
                user_name: "user_name".to_string(),
                path: "/path".to_string(),
            },
        )
    }

    pub fn test_snapshot_with_source(
        id: &str,
        total_size: u64,
        retention_reasons: &[&str],
        source: Source,
    ) -> SnapshotJson {
        SnapshotJson {
            id: id.to_string(),
            source,
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

    pub fn multi_map(
        sources_snapshots: Vec<(&str, &str, &str, Vec<SnapshotJson>)>,
    ) -> (KopiaSnapshots, Vec<SourceStr>) {
        let mut all_snapshots = Vec::new();
        let mut sources = Vec::new();

        for (user_name, host, path, snapshots) in sources_snapshots {
            let source = Source {
                host: host.to_string(),
                user_name: user_name.to_string(),
                path: path.to_string(),
            };
            let source_str = source.render().expect("valid source");
            sources.push(source_str);

            for mut snapshot in snapshots {
                snapshot.source = source.clone();
                all_snapshots.push(snapshot);
            }
        }

        let map =
            KopiaSnapshots::new_from_snapshots(all_snapshots, |_| Ok(())).expect("valid snapshots");

        (map, sources)
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        KopiaSnapshots,
        test_util::{single_map, source_str, test_snapshot},
    };

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

        let snapshots = KopiaSnapshots::new_parse_json(json, |e| eyre::bail!(e))
            .expect("valid JSON")
            .into_inner_map()
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
            test_snapshot("snap1", 1000, &["latest-1", "daily-1", "monthly-1"]),
            test_snapshot("snap2", 2000, &["latest-2", "daily-2", "monthly-2"]),
        ]);

        let counts = map
            .get_retention_counts()
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
            test_snapshot("1", 1000, &["latest-1", "daily-1"]),
            test_snapshot("2", 2000, &["daily-2"]),
        ]);

        let counts = map
            .get_retention_counts()
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

        let map = KopiaSnapshots::new_parse_json(sample_data, |e| eyre::bail!(e))
            .expect("valid snapshot JSON");

        {
            // inspect parsed snapshots (for single source)
            let snapshots = map
                .clone()
                .snapshots_map
                .into_expect_only(&source)
                .expect("single");

            assert_eq!(snapshots.len(), 17);

            let latest = snapshots.last().expect("nonempty");
            assert_eq!(latest.id, "c5be996d125abae92340f3a658443b24");
            assert_eq!(latest.start_time, "2025-08-14T00:00:04.04475167Z");
            assert_eq!(latest.stats.total_size, 42_154_950_324);
            assert_eq!(latest.stats.error_count, 0);
            assert_eq!(latest.root_entry.summ.num_failed, 0);
        }

        let retention_counts = map.get_retention_counts();
        let retention_counts = retention_counts.into_expect_only(&source).expect("single");
        assert_eq!(retention_counts.get("latest-1"), Some(&1));
        assert_eq!(retention_counts.get("daily-1"), Some(&1));
        assert_eq!(retention_counts.get("monthly-1"), Some(&1));
    }
}
