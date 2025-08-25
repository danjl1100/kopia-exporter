//! Kopia Exporter - A Prometheus metrics exporter for Kopia backup repositories.
//!
//! This application exports metrics from Kopia backup repositories in a format
//! suitable for Prometheus monitoring.

use base64::prelude::*;
use clap::Parser;
use kopia_exporter::{Snapshot, get_snapshots_from_command, metrics};
use std::time::{Duration, Instant};
use tiny_http::{Header, Method, Response, Server};

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Kopia binary path
    #[arg(short, long, default_value = "kopia")]
    kopia_bin: String,

    /// Server bind address
    #[arg(short, long, default_value = "127.0.0.1:9090")]
    bind: String,

    /// Cache duration in seconds (0 to disable)
    #[arg(short, long, default_value = "30")]
    cache_seconds: u64,

    /// Maximum number of bind retry attempts (0 = no retries, just 1 attempt)
    #[arg(short = 'r', long, default_value = "5")]
    max_bind_retries: u32,

    /// Basic auth username
    #[arg(long)]
    auth_username: Option<String>,

    /// Basic auth password
    #[arg(long)]
    auth_password: Option<String>,

    /// Path to file containing username:password for basic auth
    #[arg(long)]
    auth_credentials_file: Option<String>,
}

#[derive(Debug, Clone)]
struct BasicAuthConfig {
    username: String,
    password: String,
}

impl BasicAuthConfig {
    fn from_args(args: &Args) -> eyre::Result<Option<Self>> {
        match (
            &args.auth_username,
            &args.auth_password,
            &args.auth_credentials_file,
        ) {
            (Some(username), Some(password), None) => Ok(Some(Self {
                username: username.clone(),
                password: password.clone(),
            })),
            (None, None, Some(file_path)) => {
                let content = std::fs::read_to_string(file_path).map_err(|e| {
                    eyre::eyre!(
                        "Failed to read auth credentials file '{}': {}",
                        file_path,
                        e
                    )
                })?;
                let content = content.trim();
                if let Some((username, password)) = content.split_once(':') {
                    Ok(Some(Self {
                        username: username.to_string(),
                        password: password.to_string(),
                    }))
                } else {
                    Err(eyre::eyre!(
                        "Auth credentials file must contain 'username:password'"
                    ))
                }
            }
            (None, None, None) => Ok(None),
            _ => Err(eyre::eyre!(
                "Invalid auth configuration: use either --auth-username + --auth-password OR --auth-credentials-file, not both"
            )),
        }
    }

    fn validate_request(&self, request: &tiny_http::Request) -> bool {
        if let Some(auth_header) = request
            .headers()
            .iter()
            .find(|h| h.field.as_str() == "Authorization")
            && let Ok(auth_value) = std::str::from_utf8(auth_header.value.as_bytes())
            && let Some(credentials) = auth_value.strip_prefix("Basic ")
            && let Ok(decoded) = BASE64_STANDARD.decode(credentials)
            && let Ok(decoded_str) = std::str::from_utf8(&decoded)
        {
            let expected = format!("{}:{}", self.username, self.password);
            return decoded_str == expected;
        }
        false
    }
}

#[derive(Debug, Clone)]
struct TimedSnapshots {
    snapshots: Vec<Snapshot>,
    created_at: Instant,
}
impl TimedSnapshots {
    fn now(snapshots: Vec<Snapshot>) -> Self {
        Self {
            snapshots,
            created_at: Instant::now(),
        }
    }
}

fn send_unauthorized_response(request: tiny_http::Request) {
    let header = Header::from_bytes(
        &b"WWW-Authenticate"[..],
        &b"Basic realm=\"Kopia Exporter\""[..],
    )
    .expect("Invalid header");
    let response = Response::from_string("Unauthorized")
        .with_status_code(401)
        .with_header(header);
    let _ = request.respond(response);
}

#[allow(clippy::needless_pass_by_value)] // Server is consumed by incoming_requests()
fn serve_requests(
    server: Server,
    kopia_bin: &str,
    cache_duration: Duration,
    auth: Option<BasicAuthConfig>,
) {
    let mut cache: Option<TimedSnapshots> = None;
    for request in server.incoming_requests() {
        // Check authentication if configured
        if let Some(ref auth_config) = auth
            && !auth_config.validate_request(&request)
        {
            send_unauthorized_response(request);
            continue;
        }

        match (request.method(), request.url()) {
            (&Method::Get, "/metrics") => {
                // 1. Check if cached value is available (clear if expired)
                if let Some(cached) = &cache
                    && cached.created_at.elapsed() >= cache_duration
                {
                    cache = None; // Clear expired cache
                }

                // 2. Get snapshots (from cache or fresh fetch)
                let current = cache.take().map_or_else(
                    || get_snapshots_from_command(kopia_bin).map(TimedSnapshots::now),
                    Ok,
                );

                // 3. Serve the result
                match &current {
                    Ok(TimedSnapshots { snapshots, .. }) => {
                        let now = jiff::Timestamp::now();
                        let metrics_output = metrics::generate_all_metrics(snapshots, now);
                        let header = Header::from_bytes(
                            &b"Content-Type"[..],
                            &b"text/plain; charset=utf-8"[..],
                        )
                        .expect("Invalid header");
                        let response = Response::from_string(metrics_output).with_header(header);
                        let _ = request.respond(response);
                    }
                    Err(e) => {
                        eprintln!("Error fetching snapshots: {e}");
                        let error_response =
                            Response::from_string("Error fetching metrics").with_status_code(500);
                        let _ = request.respond(error_response);
                    }
                }

                // 4. Store result in cache (if successful and cache enabled)
                if let Ok(current) = current
                    && !cache_duration.is_zero()
                {
                    cache = Some(current);
                }
            }
            (&Method::Get, "/") => {
                let html = include_str!("index.html");
                let header =
                    Header::from_bytes(&b"Content-Type"[..], &b"text/html; charset=utf-8"[..])
                        .expect("Invalid header");
                let response = Response::from_string(html).with_header(header);
                let _ = request.respond(response);
            }
            _ => {
                let response = Response::from_string("Not Found").with_status_code(404);
                let _ = request.respond(response);
            }
        }
    }
}

fn calculate_delay_seconds(attempt: u32) -> u64 {
    (1u64 << (attempt - 1)).min(16) // 1, 2, 4, 8, 16, 16, 16... seconds (capped at 16)
}

fn start_server_with_retry(bind_addr: &str, max_retries: u32) -> eyre::Result<Server> {
    let mut attempt = 1;
    let mut retries_remaining = max_retries;

    loop {
        // 1. First attempt (or retry attempt)
        match Server::http(bind_addr) {
            Ok(server) => {
                if attempt > 1 {
                    println!("Successfully bound to {bind_addr} on attempt {attempt}");
                }
                return Ok(server);
            }
            Err(e) => {
                // 2. If fails, check retries remaining
                if retries_remaining == 0 {
                    // 4. If exhausted, return error
                    return Err(eyre::eyre!(
                        "Failed to bind to {bind_addr} after {attempt} attempts: {e}"
                    ));
                }

                // 3. If allowed, delay and continue
                let delay_secs = calculate_delay_seconds(attempt);
                eprintln!("Bind attempt {attempt} failed: {e}. Retrying in {delay_secs}s...");
                std::thread::sleep(Duration::from_secs(delay_secs));

                attempt += 1;
                retries_remaining -= 1;
            }
        }
    }
}

fn main() -> eyre::Result<()> {
    let args = Args::parse();

    let auth = BasicAuthConfig::from_args(&args)?;
    if auth.is_some() {
        println!("Basic authentication enabled");
    }

    println!("Starting Kopia Exporter on {}", args.bind);

    let server = start_server_with_retry(&args.bind, args.max_bind_retries)?;

    let cache_duration = Duration::from_secs(args.cache_seconds);
    serve_requests(server, &args.kopia_bin, cache_duration, auth);

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::TcpListener;

    #[test]
    fn test_start_server_with_retry_success_first_attempt() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        drop(listener);

        let result = start_server_with_retry(&addr.to_string(), 3);
        assert!(result.is_ok());
    }

    #[test]
    fn test_start_server_with_retry_no_retries() {
        let result = start_server_with_retry("127.0.0.1:99999", 0);
        assert!(result.is_err());
        let err_msg = result.err().unwrap().to_string();
        assert!(err_msg.contains("Failed to bind to 127.0.0.1:99999"));
        assert!(err_msg.contains("after 1 attempts")); // 0 retries = 1 attempt only
    }

    #[test]
    fn test_start_server_with_retry_exhausted() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();

        let result = start_server_with_retry(&addr.to_string(), 2);
        assert!(result.is_err());
        let err_msg = result.err().unwrap().to_string();
        assert!(err_msg.contains("Failed to bind to"));
        assert!(err_msg.contains("after 3 attempts")); // 1 initial + 2 retries = 3 attempts
    }

    #[test]
    fn test_delay_calculation_and_cap() {
        // Test exponential backoff sequence
        assert_eq!(calculate_delay_seconds(1), 1); // 2^0 = 1
        assert_eq!(calculate_delay_seconds(2), 2); // 2^1 = 2  
        assert_eq!(calculate_delay_seconds(3), 4); // 2^2 = 4
        assert_eq!(calculate_delay_seconds(4), 8); // 2^3 = 8
        assert_eq!(calculate_delay_seconds(5), 16); // 2^4 = 16

        // Test cap at 16 seconds
        assert_eq!(calculate_delay_seconds(6), 16); // 2^5 = 32, but capped at 16
        assert_eq!(calculate_delay_seconds(7), 16); // 2^6 = 64, but capped at 16
        assert_eq!(calculate_delay_seconds(10), 16); // 2^9 = 512, but capped at 16

        // Verify total delay for common retry counts
        let total_delay_5_retries: u64 = (1..=6).map(calculate_delay_seconds).sum(); // 1+2+4+8+16+16=47s
        assert_eq!(total_delay_5_retries, 47);

        // Without cap, 6 attempts would be: 1+2+4+8+16+32=63s
        // With cap: 1+2+4+8+16+16=47s (16s saved)
    }
}
