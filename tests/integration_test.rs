//! Integration tests for kopia-exporter functionality.

use eyre::Result;
use kopia_exporter::kopia;
use std::fs;
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

#[test]
fn test_caching_reduces_subprocess_calls() -> Result<()> {
    let fake_kopia_bin = env!("CARGO_BIN_EXE_fake-kopia");
    let kopia_exporter_bin = env!("CARGO_BIN_EXE_kopia-exporter");

    // Test with caching enabled (1 second cache for quick testing)
    let log_file_cached = format!("/tmp/fake-kopia-cache-test-{}.log", std::process::id());

    let mut server_process_cached = Command::new(kopia_exporter_bin)
        .args([
            "--kopia-bin",
            fake_kopia_bin,
            "--bind",
            "127.0.0.1:9093",
            "--cache-seconds",
            "1",
        ])
        .env("FAKE_KOPIA_LOG", &log_file_cached)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()?;

    thread::sleep(Duration::from_millis(500));

    // Make 3 rapid requests
    for _ in 0..3 {
        let _ = minreq::get("http://127.0.0.1:9093/metrics").send()?;
        thread::sleep(Duration::from_millis(50)); // Small delay between requests
    }

    server_process_cached.kill()?;
    server_process_cached.wait()?;

    // Count invocations with caching
    let cached_log = fs::read_to_string(&log_file_cached).unwrap_or_default();
    let cached_calls = cached_log.lines().count();

    // Test with caching disabled
    let log_file_no_cache = format!("/tmp/fake-kopia-no-cache-test-{}.log", std::process::id());

    let mut server_process_no_cache = Command::new(kopia_exporter_bin)
        .args([
            "--kopia-bin",
            fake_kopia_bin,
            "--bind",
            "127.0.0.1:9094",
            "--cache-seconds",
            "0",
        ])
        .env("FAKE_KOPIA_LOG", &log_file_no_cache)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()?;

    thread::sleep(Duration::from_millis(500));

    // Make 3 rapid requests
    for _ in 0..3 {
        let _ = minreq::get("http://127.0.0.1:9094/metrics").send()?;
        thread::sleep(Duration::from_millis(50));
    }

    server_process_no_cache.kill()?;
    server_process_no_cache.wait()?;

    // Count invocations without caching
    let no_cache_log = fs::read_to_string(&log_file_no_cache).unwrap_or_default();
    let no_cache_calls = no_cache_log.lines().count();

    // Clean up log files
    let _ = fs::remove_file(&log_file_cached);
    let _ = fs::remove_file(&log_file_no_cache);

    // With caching, we should see only 1 call to fake-kopia
    // Without caching, we should see 3 calls
    assert_eq!(
        cached_calls, 1,
        "Expected 1 kopia call with caching enabled"
    );
    assert_eq!(
        no_cache_calls, 3,
        "Expected 3 kopia calls with caching disabled"
    );

    Ok(())
}
