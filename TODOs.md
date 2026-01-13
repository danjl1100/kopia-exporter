When writing the top-level crate documentation in `src/lib.rs`, a few metric names were listed that are not actually implemented.
We want to figure out how best to avoid this kind of in the future, in addition to removing the unimplemented entries.

Tasks:
- [ ] propose a few different approaches for ensuring the crate-level docs only list implemented metrics

---

Once an implementation plan is agreed upon:
- [ ] remove the unimplemented entries:
    - `kopia_repository_accessible`
    - `kopia_snapshot_duration_seconds`
    - `kopia_snapshot_throughput_bytes_per_second`
    - `kopia_retention_policy_violations_total`
- [ ] update the documentation per the plan to future-proof against listing unimplemented metric names

