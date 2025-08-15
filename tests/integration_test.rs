//! Integration tests for kopia-exporter functionality.

use eyre::Result;
use kopia_exporter::kopia;
use std::process::{Command, Stdio};
use std::thread;
use std::time::Duration;

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

#[test]
fn test_web_server_integration() -> Result<()> {
    let fake_kopia_bin = env!("CARGO_BIN_EXE_fake-kopia");
    let kopia_exporter_bin = env!("CARGO_BIN_EXE_kopia-exporter");

    // Start the web server in the background
    let mut server_process = Command::new(kopia_exporter_bin)
        .args(["--kopia-bin", fake_kopia_bin, "--bind", "127.0.0.1:9092"])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()?;

    // Wait for server to start
    thread::sleep(Duration::from_millis(500));

    // Test the root endpoint
    let root_response = minreq::get("http://127.0.0.1:9092/").send()?;

    assert_eq!(root_response.status_code, 200);
    let root_text = root_response.as_str()?;
    assert!(root_text.contains("Kopia Exporter"));
    assert!(root_text.contains("/metrics"));

    // Test the metrics endpoint
    let metrics_response = minreq::get("http://127.0.0.1:9092/metrics").send()?;

    assert_eq!(metrics_response.status_code, 200);
    let metrics_text = metrics_response.as_str()?;

    // Verify Prometheus metrics format and content
    assert!(metrics_text.contains("# HELP kopia_snapshots_by_retention"));
    assert!(metrics_text.contains("# TYPE kopia_snapshots_by_retention gauge"));
    assert!(metrics_text.contains("kopia_snapshots_by_retention{retention_reason=\"latest-1\"} 1"));

    assert!(metrics_text.contains("# HELP kopia_snapshot_total_size_bytes"));
    assert!(metrics_text.contains("# TYPE kopia_snapshot_total_size_bytes gauge"));
    assert!(metrics_text.contains("kopia_snapshot_total_size_bytes 42154950324"));

    // Test 404 endpoint
    let not_found_response = minreq::get("http://127.0.0.1:9092/nonexistent").send()?;

    assert_eq!(not_found_response.status_code, 404);

    // Clean up: terminate the server process
    server_process.kill()?;
    server_process.wait()?;

    Ok(())
}
