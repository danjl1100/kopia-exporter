# kopia-exporter

## Goal

As a self-hosted backup user/operator, there are several aspects of backup that are easy to miss.

Step one is to automate the backup, but how to you ensure it stays healthy over time?

Monitoring for an unattended backup should verify these key tenants:
- New snapshot health
    - the newest snapshot should be no older than a specific time threshold
- Remaining space
    - `kopia` may not report the free space, but measuring the change in total space used can signal configuration errors (tracking too many large frequently change files)
- Pruned snapshots
    - The oldest snapshots should
- Pruning health

## Metrics

TODO list of metrics

## Development

This project is fully independent from any real `kopia` setup, so a fake kopia binary is included for testing to provide realistic test scenarios.
