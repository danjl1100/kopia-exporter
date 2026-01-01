//! Common helper functions for integration tests.

use eyre::Result;
use std::net::TcpListener;
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::thread;
use std::time::Duration;

/// Configuration for starting a test server process.
pub struct ServerConfig {
    command: Command,
    bind_address: String,
}

impl ServerConfig {
    /// Create a basic server configuration.
    pub fn new(fake_kopia_bin: &str) -> Result<Self> {
        let kopia_exporter_bin = env!("CARGO_BIN_EXE_kopia-exporter");
        let bind_address = get_test_bind_address()?;

        let mut command = Command::new(kopia_exporter_bin);
        command
            .args(["--kopia-bin", fake_kopia_bin, "--bind", &bind_address])
            .stdout(Stdio::null())
            .stderr(Stdio::null());

        Ok(Self {
            command,
            bind_address,
        })
    }

    /// Add additional command line arguments.
    pub fn with_args<I, S>(mut self, args: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: AsRef<std::ffi::OsStr>,
    {
        self.command.args(args);
        self
    }

    /// Add environment variables.
    pub fn with_env<K, V>(mut self, key: K, val: V) -> Self
    where
        K: AsRef<std::ffi::OsStr>,
        V: AsRef<std::ffi::OsStr>,
    {
        self.command.env(key, val);
        self
    }
}

/// Helper for managing test server processes.
pub struct TestServer {
    process: Child,
    bind_address: String,
}

impl TestServer {
    /// Start a new test server with the given configuration.
    pub fn start(mut config: ServerConfig) -> Result<Self> {
        let process = config.command.spawn()?;
        let bind_address = config.bind_address;

        // Wait for server to start
        thread::sleep(Duration::from_millis(500));

        Ok(Self {
            process,
            bind_address,
        })
    }

    /// Make an HTTP GET request to the server.
    pub fn get(&self, path: &str) -> Result<minreq::Response> {
        let url = format!("http://{}{}", self.bind_address, path);
        Ok(minreq::get(&url).send()?)
    }

    /// Make an authenticated HTTP GET request to the server.
    pub fn get_with_auth(&self, path: &str, auth_header: &str) -> Result<minreq::Response> {
        let url = format!("http://{}{}", self.bind_address, path);
        Ok(minreq::get(&url)
            .with_header("Authorization", auth_header)
            .send()?)
    }
}

impl Drop for TestServer {
    fn drop(&mut self) {
        let _ = self.process.kill();
        let _ = self.process.wait();
    }
}

/// Common assertions for HTTP responses.
pub mod assertions {
    use eyre::Result;

    /// Assert that a response contains expected Prometheus metrics.
    pub fn assert_prometheus_metrics(response_text: &str) -> Result<()> {
        assert!(response_text.contains("# HELP kopia_snapshots_by_retention"));
        assert!(response_text.contains("# TYPE kopia_snapshots_by_retention gauge"));
        assert!(
            response_text.contains("kopia_snapshots_by_retention{retention_reason=\"latest-1\"} 1")
        );

        assert!(response_text.contains("# HELP kopia_snapshot_total_size_bytes"));
        assert!(response_text.contains("# TYPE kopia_snapshot_total_size_bytes gauge"));
        assert!(response_text.contains("kopia_snapshot_total_size_bytes 42154950324"));

        Ok(())
    }

    /// Assert that a response contains the root page content.
    pub fn assert_root_page_content(response_text: &str) -> Result<()> {
        assert!(response_text.contains("Kopia Exporter"));
        assert!(response_text.contains("/metrics"));
        Ok(())
    }
}

/// Get a random available port from the OS for testing.
pub fn get_test_bind_address() -> Result<String> {
    let listener = TcpListener::bind("127.0.0.1:0")?;
    let addr = listener.local_addr()?;
    let port = addr.port();
    drop(listener);
    Ok(format!("127.0.0.1:{}", port))
}

/// Generate a unique log file path for testing.
///
/// # Panics
/// Panics if creating the temporary directory fails
pub fn get_test_log_path(suffix: &str) -> (tempfile::TempDir, PathBuf) {
    let temp_dir = tempfile::tempdir().expect("test failed to create TempDir");
    let path = temp_dir.path().join(format!("fake-kopia-{}.log", suffix));
    (temp_dir, path)
}
