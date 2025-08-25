//! Integration tests for kopia-exporter functionality.

use eyre::Result;
use kopia_exporter::kopia;
use std::fs;
use std::thread;
use std::time::Duration;

mod test_helpers;
use test_helpers::{ServerConfig, TestServer, assertions, get_test_log_path};

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
    let config = ServerConfig::new(fake_kopia_bin)?;
    let server = TestServer::start(config)?;

    // Test the root endpoint
    let root_response = server.get("/")?;
    assert_eq!(root_response.status_code, 200);
    assertions::assert_root_page_content(root_response.as_str()?)?;

    // Test the metrics endpoint
    let metrics_response = server.get("/metrics")?;
    assert_eq!(metrics_response.status_code, 200);
    assertions::assert_prometheus_metrics(metrics_response.as_str()?)?;

    // Test 404 endpoint
    let not_found_response = server.get("/nonexistent")?;
    assert_eq!(not_found_response.status_code, 404);

    Ok(())
}

#[test]
fn test_caching_reduces_subprocess_calls() -> Result<()> {
    let fake_kopia_bin = env!("CARGO_BIN_EXE_fake-kopia");

    // Test with caching enabled (1 second cache for quick testing)
    let log_file_cached = get_test_log_path("cache");
    let cached_config = ServerConfig::new(fake_kopia_bin)?
        .with_args(["--cache-seconds", "1"])
        .with_env("FAKE_KOPIA_LOG", &log_file_cached);
    let cached_server = TestServer::start(cached_config)?;

    // Make 3 rapid requests
    for _ in 0..3 {
        let _ = cached_server.get("/metrics")?;
        thread::sleep(Duration::from_millis(50));
    }
    drop(cached_server);

    // Count invocations with caching
    let cached_log = fs::read_to_string(&log_file_cached).unwrap_or_default();
    let cached_calls = cached_log.lines().count();

    // Test with caching disabled
    let log_file_no_cache = get_test_log_path("no-cache");
    let no_cache_config = ServerConfig::new(fake_kopia_bin)?
        .with_args(["--cache-seconds", "0"])
        .with_env("FAKE_KOPIA_LOG", &log_file_no_cache);
    let no_cache_server = TestServer::start(no_cache_config)?;

    // Make 3 rapid requests
    for _ in 0..3 {
        let _ = no_cache_server.get("/metrics")?;
        thread::sleep(Duration::from_millis(50));
    }
    drop(no_cache_server);

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

#[test]
fn test_basic_auth_integration() -> Result<()> {
    let fake_kopia_bin = env!("CARGO_BIN_EXE_fake-kopia");
    let config = ServerConfig::new(fake_kopia_bin)?.with_args([
        "--auth-username",
        "testuser",
        "--auth-password",
        "testpass",
    ]);
    let server = TestServer::start(config)?;

    // Test unauthenticated request - should get 401
    let unauth_response = server.get("/metrics")?;
    assert_eq!(unauth_response.status_code, 401);
    assert!(unauth_response.headers.get("www-authenticate").is_some());

    // Test with correct credentials
    let auth_response = server.get_with_auth("/metrics", "Basic dGVzdHVzZXI6dGVzdHBhc3M=")?; // testuser:testpass
    assert_eq!(auth_response.status_code, 200);
    assertions::assert_prometheus_metrics(auth_response.as_str()?)?;

    // Test with incorrect credentials
    let bad_auth_response = server.get_with_auth("/metrics", "Basic aW52YWxpZDppbnZhbGlk")?; // invalid:invalid
    assert_eq!(bad_auth_response.status_code, 401);

    Ok(())
}

#[test]
fn test_basic_auth_credentials_file_integration() -> Result<()> {
    use std::io::Write;

    let fake_kopia_bin = env!("CARGO_BIN_EXE_fake-kopia");

    // Create temporary credentials file
    let mut temp_file = tempfile::NamedTempFile::new()?;
    writeln!(temp_file, "fileuser:filepass")?;
    let temp_path = temp_file.path().to_string_lossy().to_string();

    let config =
        ServerConfig::new(fake_kopia_bin)?.with_args(["--auth-credentials-file", &temp_path]);
    let server = TestServer::start(config)?;

    // Test unauthenticated request - should get 401
    let unauth_response = server.get("/metrics")?;
    assert_eq!(unauth_response.status_code, 401);

    // Test with correct credentials from file
    let auth_response = server.get_with_auth("/metrics", "Basic ZmlsZXVzZXI6ZmlsZXBhc3M=")?; // fileuser:filepass
    assert_eq!(auth_response.status_code, 200);
    assertions::assert_prometheus_metrics(auth_response.as_str()?)?;

    // Test with incorrect credentials
    let bad_auth_response =
        server.get_with_auth("/metrics", "Basic d3JvbmdVc2VyOndyb25nUGFzcw==")?; // wrongUser:wrongPass
    assert_eq!(bad_auth_response.status_code, 401);

    Ok(())
}
