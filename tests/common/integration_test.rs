//! Integration tests for kopia-exporter functionality.

#![expect(clippy::unwrap_used)] // tests can unwrap

use crate::FAKE_KOPIA_BIN;
use crate::test_helpers::{ServerConfig, TestServer, assertions, get_test_log_path};
use eyre::Result;
use kopia_exporter::{KopiaSnapshots, SourceStr};
use std::fs;
use std::thread;
use std::time::Duration;

#[test]
fn test_subprocess_with_fake_kopia() {
    let timeout = std::time::Duration::from_secs(15);

    let source = SourceStr::new_unchecked("kopia-system@milton:/persist-home".to_string());

    let snapshots =
        KopiaSnapshots::new_from_command(FAKE_KOPIA_BIN, timeout, |e| eyre::bail!(e)).unwrap();

    let retention_counts = snapshots
        .get_retention_counts()
        .into_expect_only(&source)
        .expect("single");
    assert_eq!(retention_counts.get("latest-1"), Some(&1));

    let snapshots = snapshots
        .into_inner_map()
        .into_expect_only(&source)
        .expect("single");
    assert_eq!(snapshots.len(), 17);

    if let Some(latest) = snapshots.last() {
        assert_eq!(latest.id, "c5be996d125abae92340f3a658443b24");
        assert_eq!(latest.stats.error_count, 0);
    }
}

#[test]
fn test_web_server_integration() -> Result<()> {
    let config = ServerConfig::new(FAKE_KOPIA_BIN)?;
    let server = TestServer::start(config)?;

    // Test the root endpoint
    let root_response = server.get("/")?;
    assert_eq!(root_response.status_code, 200);
    assertions::assert_root_page_content(root_response.as_str()?);

    // Test the metrics endpoint
    let metrics_response = server.get("/metrics")?;
    assert_eq!(metrics_response.status_code, 200);
    assertions::assert_prometheus_metrics(metrics_response.as_str()?);

    // Test 404 endpoint
    let not_found_response = server.get("/nonexistent")?;
    assert_eq!(not_found_response.status_code, 404);

    Ok(())
}

#[test]
fn test_caching_reduces_subprocess_calls() -> Result<()> {
    // Test with caching enabled (1 second cache for quick testing)
    let (_tempdir, log_file_cached) = get_test_log_path("cache");
    let cached_config = ServerConfig::new(FAKE_KOPIA_BIN)?
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
    let (_tempdir, log_file_no_cache) = get_test_log_path("no-cache");
    let no_cache_config = ServerConfig::new(FAKE_KOPIA_BIN)?
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
    let config = ServerConfig::new(FAKE_KOPIA_BIN)?.with_args([
        "--auth-username",
        "testuser",
        "--auth-password",
        "testpass",
    ]);
    let server = TestServer::start(config)?;

    // Test unauthenticated request - should get 401
    let unauth_response = server.get("/metrics")?;
    assert_eq!(unauth_response.status_code, 401);
    assert!(unauth_response.headers.contains_key("www-authenticate"));

    // Test with correct credentials
    let auth_response = server.get_with_auth("/metrics", "Basic dGVzdHVzZXI6dGVzdHBhc3M=")?; // testuser:testpass
    assert_eq!(auth_response.status_code, 200);
    assertions::assert_prometheus_metrics(auth_response.as_str()?);

    // Test with incorrect credentials
    let bad_auth_response = server.get_with_auth("/metrics", "Basic aW52YWxpZDppbnZhbGlk")?; // invalid:invalid
    assert_eq!(bad_auth_response.status_code, 401);

    Ok(())
}

#[test]
fn test_basic_auth_credentials_file_integration() -> Result<()> {
    use std::io::Write;

    // Create temporary credentials file
    let mut temp_file = tempfile::NamedTempFile::new()?;
    writeln!(temp_file, "fileuser:filepass")?;
    let temp_path = temp_file.path().to_string_lossy().to_string();

    let config =
        ServerConfig::new(FAKE_KOPIA_BIN)?.with_args(["--auth-credentials-file", &temp_path]);
    let server = TestServer::start(config)?;

    // Test unauthenticated request - should get 401
    let unauth_response = server.get("/metrics")?;
    assert_eq!(unauth_response.status_code, 401);

    // Test with correct credentials from file
    let auth_response = server.get_with_auth("/metrics", "Basic ZmlsZXVzZXI6ZmlsZXBhc3M=")?; // fileuser:filepass
    assert_eq!(auth_response.status_code, 200);
    assertions::assert_prometheus_metrics(auth_response.as_str()?);

    // Test with incorrect credentials
    let bad_auth_response =
        server.get_with_auth("/metrics", "Basic d3JvbmdVc2VyOndyb25nUGFzcw==")?; // wrongUser:wrongPass
    assert_eq!(bad_auth_response.status_code, 401);

    Ok(())
}

/// Helper function to test kopia timeout behavior with different sleep values.
fn run_timeout_test(
    sleep_value: &str,
    expected_log_content: &str,
    test_suffix: &str,
) -> Result<()> {
    // Setup: log file to verify sleep parameter was passed correctly
    let (_tempdir, log_file) = get_test_log_path(test_suffix);

    // Configure server with specified sleep and 0.5 second timeout
    let config = ServerConfig::new(FAKE_KOPIA_BIN)?
        .with_env("FAKE_KOPIA_SLEEP_FOR_SECS", sleep_value)
        .with_env("FAKE_KOPIA_LOG", &log_file)
        .with_args(["--timeout", "0.5"]);
    let server = TestServer::start(config)?;

    // Request metrics - should timeout and return 500
    let response = server.get("/metrics")?;
    assert_eq!(
        response.status_code, 500,
        "expect HTTP 500 for sleep {sleep_value:?}"
    );

    // Verify the sleep parameter was actually set (avoid false pass)
    let log_content = fs::read_to_string(&log_file).unwrap_or_default();
    assert!(
        log_content.contains(expected_log_content),
        "Expected log to show {expected_log_content}, got: {log_content}"
    );

    Ok(())
}

#[test]
fn test_kopia_timeout_returns_500() -> Result<()> {
    run_timeout_test("1", "ForSecs(1.0)", "timeout")
}

#[test]
fn test_kopia_timeout_forever_returns_500() -> Result<()> {
    run_timeout_test("forever", "Forever", "timeout-forever")
}

#[test]
fn test_timeout_prints_stdout_and_stderr() -> Result<()> {
    // Configure server to trigger timeout and capture stderr
    let config = ServerConfig::new(FAKE_KOPIA_BIN)?
        .with_args(["--timeout", "0.1"])
        .with_env("FAKE_KOPIA_WRITE_TEST_OUTPUT", "1")
        .with_env("FAKE_KOPIA_SLEEP_FOR_SECS", "10")
        .with_stderr_capture();

    let server = TestServer::start(config)?;

    // Make request that will timeout
    let response = server.get("/metrics")?;
    assert_eq!(response.status_code, 500, "Expected HTTP 500 on timeout");

    // Kill server and read its stderr
    let stderr_output = server.kill_and_read_stderr();

    // Verify stderr contains the error message with stdout/stderr from fake-kopia
    assert!(
        stderr_output.contains("fake-kopia-test-stdout"),
        "Server stderr should contain fake-kopia stdout. Stderr: {stderr_output}"
    );
    assert!(
        stderr_output.contains("fake-kopia-test-stderr"),
        "Server stderr should contain fake-kopia stderr. Stderr: {stderr_output}"
    );

    Ok(())
}
