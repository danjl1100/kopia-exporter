- [x] Let's brainstorm then implement the best option for this issue: Other prometheus exporter services on my desktop computer have trouble binding to a port on the tailscale interface, specifically
    - the prometheus smartctl exporter requires setting `systemd.services.<name>.after` and `bindsTo` with the interface.
    - another custom Rust exporter like this one needs both `after`, `bindsTo`, but also needs an `ExecStartPre` script that verifies the tailscale address is pingable, otherwise it sees spurious bind issues after the desktop resumes from sleep.
    - I'm wondering if the prometheus-written exporter has extra logic to retry if the port binding fails, or if that's even a viable option.
    **Decision: Application-level retry logic with exponential backoff**

- [x] Add `--max-bind-retries` CLI flag (default: 5)
- [x] Implement exponential backoff retry logic for server binding (1s, 2s, 4s, 8s, 16s)
- [x] Add logging for each bind attempt for debugging
- [x] Write unit tests for retry logic
- [x] Write integration tests for bind failure scenarios
- [x] Update help text and documentation

## cleanup
- [x] update `tests/bind_retry_test.rs` to use `env!("CARGO_BIN_EXE_kopia-exporter")` instead of cargo
