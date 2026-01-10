- [x] add tests for metrics modules that are missing tests (marked as an ignored `todo` tests), verify the metric calculation through inspecting the output metric strings
    - see existing metric tests for good examples in metrics modules: `total_size_bytes` and `retention`

------

- [x] add a metric `kopia_snapshot_source_parse_errors` (similar to `kopia_snapshot_timestamp_parse_errors_total`), that reports how many snapshots had invalid sources
    - include a labels with the invalid data (examples in item below)
- [x] add a test similar to `test_snapshot_age_metric_invalid_time` that tests the invalid `user_name` and `host` field handling
    - example: expect `kopia_snapshot_source_parse_errors{invalid_user="ba@d_username"} 2`, if there are 2 snapshots with that invalid username
    - example: expect `kopia_snapshot_source_parse_errors{invalid_host="server_col:on"} 5`, if there are 5 snapshots with that invalid hostname
