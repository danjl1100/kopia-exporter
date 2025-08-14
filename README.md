# kopia-exporter

## Goal

As a self-hosted backup user/operator, there are several aspects of backup that are easy to miss.

Step one is to automate the backup, but how do you ensure it stays healthy over time?

Monitoring for an unattended backup should verify these key tenets:
- New snapshot health
    - the newest snapshot should be no older than a specific time threshold
- Backup completion status
    - verify that backup jobs complete successfully without errors
- Data integrity verification
    - ensure snapshots are readable and restorable
- Repository connectivity
    - confirm connection to backup destination is maintained
- Performance metrics
    - track backup duration and throughput for performance degradation
- Remaining space
    - `kopia` may not report free space directly, but measuring changes in total space used can signal configuration errors
- Pruned snapshots
    - The oldest snapshots should be pruned according to retention policy
- Pruning health
    - Verify that pruning operations complete successfully and maintain expected retention

## Metrics

### New snapshot health
- `kopia_snapshot_age_seconds` - Age of newest snapshot in seconds
- `kopia_snapshot_last_success_timestamp` - Unix timestamp of last successful snapshot

### Backup completion status
- `kopia_snapshot_errors_total` - Total errors in latest snapshot
- `kopia_snapshot_ignored_errors_total` - Ignored errors in latest snapshot

### Data integrity verification
- `kopia_snapshot_failed_files_total` - Number of failed files in latest snapshot

### Repository connectivity
- `kopia_repository_accessible` - 1 if repository is accessible, 0 otherwise

### Performance metrics
- `kopia_snapshot_duration_seconds` - Backup duration (endTime - startTime)
- `kopia_snapshot_throughput_bytes_per_second` - Bytes per second throughput

### Remaining space
- `kopia_snapshot_total_size_bytes` - Total size of snapshot in bytes
- `kopia_snapshot_size_change_bytes` - Change in size from previous snapshot

### Pruned snapshots
- `kopia_snapshots_total` - Total number of snapshots
- `kopia_snapshots_by_retention` - Number of snapshots by retention reason (labeled)

### Pruning health
- `kopia_retention_policy_violations_total` - Snapshots that should have been pruned but weren't

## Development

This project is fully independent from any real `kopia` setup, so a fake kopia binary is included for testing to provide realistic test scenarios.

### Architecture

All core logic is implemented in the library crate (`src/lib.rs`), keeping the main binary lean and focused on CLI argument handling. This design allows for easy testing and potential future expansion (e.g., web server interface).
