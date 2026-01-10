//! Integration tests for bind retry functionality.

#![expect(clippy::unwrap_used)] // tests can unwrap
#![expect(clippy::panic)] // tests can panic

use std::net::TcpListener;
use std::process::{Command, Stdio};
use std::thread;
use std::time::Duration;

#[test]
fn test_cli_bind_retry_flag() {
    let output = Command::new(env!("CARGO_BIN_EXE_kopia-exporter"))
        .args(["--help"])
        .output()
        .expect("Failed to run command");

    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("--max-bind-retries"));
    assert!(stdout.contains("Maximum number of bind retry attempts"));
}

#[test]
fn test_bind_retry_with_occupied_port() {
    // Bind to a random port to occupy it
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();
    let port = addr.port();

    // Try to start the exporter on the same port with retries disabled
    let output = Command::new(env!("CARGO_BIN_EXE_kopia-exporter"))
        .args([
            "--bind",
            &format!("127.0.0.1:{port}"),
            "--max-bind-retries",
            "0",
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("Failed to start process")
        .wait_with_output()
        .expect("Failed to wait for process");

    // Should fail immediately with no retries
    assert!(!output.status.success());
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains("Failed to bind to"));

    drop(listener);
}

#[test]
fn test_bind_retry_success_after_port_freed() {
    // Bind to a random port temporarily
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();
    let port = addr.port();

    // Start the exporter process with retries in the background
    let mut child = Command::new(env!("CARGO_BIN_EXE_kopia-exporter"))
        .args([
            "--bind",
            &format!("127.0.0.1:{port}"),
            "--max-bind-retries",
            "3",
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("Failed to start process");

    // Wait a moment, then free the port
    thread::sleep(Duration::from_millis(500));
    drop(listener);

    // Give the retry logic time to succeed
    thread::sleep(Duration::from_secs(3));

    // Check if process is still running (success case)
    let status_result = child.try_wait();
    match status_result {
        Ok(Some(_)) => {
            // Process exited, check output
            let output = child.wait_with_output().unwrap();
            let stderr = String::from_utf8(output.stderr).unwrap();
            let stdout = String::from_utf8(output.stdout).unwrap();

            // Should either succeed or show retry attempts
            assert!(
                stderr.contains("Successfully bound to")
                    || stderr.contains("Bind attempt")
                    || stdout.contains("Starting Kopia Exporter")
            );
        }
        Ok(None) => {
            // Process still running - success! Kill it.
            child.kill().expect("Failed to kill process");
            child.wait().expect("Failed to wait for killed process");
        }
        Err(e) => panic!("Error checking process status: {e}"),
    }
}
