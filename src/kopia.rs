use eyre::{Result, eyre};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::process::Command;

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

#[derive(Debug, Clone, Serialize, Deserialize)]
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
/// Returns an error if the JSON content cannot be parsed as snapshot data.
pub fn parse_snapshots(json_content: &str) -> Result<Vec<Snapshot>> {
    Ok(serde_json::from_str(json_content)?)
}

#[must_use]
pub fn get_retention_counts(snapshots: &[Snapshot]) -> HashMap<String, u32> {
    let mut counts = HashMap::new();

    for snapshot in snapshots {
        for reason in &snapshot.retention_reason {
            *counts.entry(reason.clone()).or_insert(0) += 1;
        }
    }

    counts
}

/// Executes kopia command to retrieve snapshots and parses the output.
///
/// # Errors
///
/// Returns an error if:
/// - The kopia command fails to execute
/// - The command returns a non-zero exit code
/// - The output cannot be parsed as UTF-8
/// - The JSON output cannot be parsed as snapshot data
pub fn get_snapshots_from_command(kopia_bin: &str) -> Result<Vec<Snapshot>> {
    let output = Command::new(kopia_bin)
        .args(["snapshot", "list", "--json"])
        .output()?;

    if !output.status.success() {
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(eyre!(
            "kopia command failed with exit code: {}\nstdout: {}\nstderr: {}",
            output.status.code().unwrap_or(-1),
            stdout,
            stderr
        ));
    }

    let stdout = String::from_utf8(output.stdout)?;
    let snapshots = parse_snapshots(&stdout)?;
    Ok(snapshots)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_snapshot(id: &str, total_size: u64, retention_reasons: Vec<&str>) -> Snapshot {
        Snapshot {
            id: id.to_string(),
            source: Source {
                host: "test".to_string(),
                user_name: "user".to_string(),
                path: "/test".to_string(),
            },
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
            retention_reason: retention_reasons.iter().map(|s| s.to_string()).collect(),
        }
    }

    #[test]
    fn test_parse_single_snapshot() {
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

        let snapshots = parse_snapshots(json).unwrap();
        assert_eq!(snapshots.len(), 1);
        assert_eq!(snapshots[0].id, "test123");
        assert_eq!(snapshots[0].stats.total_size, 1000);
        assert_eq!(snapshots[0].retention_reason, vec!["latest-1", "daily-1"]);
    }

    #[test]
    fn test_retention_counts_with_multiple_slots() {
        // Test case addressing the TODO: verify retention slot counting works correctly
        // This demonstrates that monthly-1, monthly-2, etc. should be counted separately
        // because they represent different retention slots, not multiple instances
        let snapshots = vec![
            create_test_snapshot("snap1", 1000, vec!["latest-1", "daily-1", "monthly-1"]),
            create_test_snapshot("snap2", 2000, vec!["latest-2", "daily-2", "monthly-2"]),
        ];

        let counts = get_retention_counts(&snapshots);

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
    fn test_retention_counts() {
        let snapshots = vec![
            create_test_snapshot("1", 1000, vec!["latest-1", "daily-1"]),
            create_test_snapshot("2", 2000, vec!["daily-2"]),
        ];

        let counts = get_retention_counts(&snapshots);
        assert_eq!(counts.get("latest-1"), Some(&1));
        assert_eq!(counts.get("daily-1"), Some(&1));
        assert_eq!(counts.get("daily-2"), Some(&1));
    }

    #[test]
    #[expect(clippy::unreadable_literal)]
    fn test_parse_sample_data() {
        let sample_data = include_str!("sample_kopia-snapshot-list.json");
        let snapshots = parse_snapshots(sample_data).unwrap();

        assert_eq!(snapshots.len(), 17);

        if let Some(latest) = snapshots.last() {
            assert_eq!(latest.id, "c5be996d125abae92340f3a658443b24");
            assert_eq!(latest.start_time, "2025-08-14T00:00:04.04475167Z");
            assert_eq!(latest.stats.total_size, 42154950324);
            assert_eq!(latest.stats.error_count, 0);
            assert_eq!(latest.root_entry.summ.num_failed, 0);
        }

        let retention_counts = get_retention_counts(&snapshots);
        assert_eq!(retention_counts.get("latest-1"), Some(&1));
        assert_eq!(retention_counts.get("daily-1"), Some(&1));
        assert_eq!(retention_counts.get("monthly-1"), Some(&1));
    }
}
