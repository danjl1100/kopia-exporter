use kopia_exporter::kopia;

#[test]
fn test_subprocess_with_fake_kopia() {
    let fake_kopia_bin = env!("CARGO_BIN_EXE_fake-kopia");
    let snapshots = kopia::get_snapshots_from_command(fake_kopia_bin).unwrap();

    assert_eq!(snapshots.len(), 17);

    if let Some(latest) = snapshots.last() {
        assert_eq!(latest.id, "c5be996d125abae92340f3a658443b24");
        assert_eq!(latest.stats.error_count, 0);
    }

    let retention_counts = kopia::get_retention_counts(&snapshots);
    assert_eq!(retention_counts.get("latest-1"), Some(&1));
}
