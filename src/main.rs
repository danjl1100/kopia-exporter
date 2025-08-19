//! Kopia Exporter - A Prometheus metrics exporter for Kopia backup repositories.
//!
//! This application exports metrics from Kopia backup repositories in a format
//! suitable for Prometheus monitoring.

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

    /// Maximum number of bind retry attempts (0 to disable retries)
    #[arg(short = 'r', long, default_value = "5")]
    max_bind_retries: u32,
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

#[allow(clippy::needless_pass_by_value)] // Server is consumed by incoming_requests()
fn serve_requests(server: Server, kopia_bin: &str, cache_duration: Duration) {
    let mut cache: Option<TimedSnapshots> = None;
    for request in server.incoming_requests() {
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

fn start_server_with_retry(bind_addr: &str, max_retries: u32) -> eyre::Result<Server> {
    if max_retries == 0 {
        return Server::http(bind_addr)
            .map_err(|e| eyre::eyre!("Failed to bind to {}: {}", bind_addr, e));
    }

    let mut last_error = None;
    for attempt in 1..=max_retries {
        match Server::http(bind_addr) {
            Ok(server) => {
                if attempt > 1 {
                    println!("Successfully bound to {bind_addr} on attempt {attempt}");
                }
                return Ok(server);
            }
            Err(e) => {
                last_error = Some(e);
                if attempt < max_retries {
                    let delay_secs = 1u64 << (attempt - 1); // 1, 2, 4, 8, 16 seconds
                    let error_ref = last_error.as_ref().expect("last_error should be set");
                    eprintln!(
                        "Bind attempt {attempt} failed: {error_ref}. Retrying in {delay_secs}s..."
                    );
                    std::thread::sleep(Duration::from_secs(delay_secs));
                }
            }
        }
    }

    Err(eyre::eyre!(
        "Failed to bind to {bind_addr} after {max_retries} attempts: {}",
        last_error.expect("last_error should be set after loop")
    ))
}

fn main() -> eyre::Result<()> {
    let args = Args::parse();

    println!("Starting Kopia Exporter on {}", args.bind);

    let server = start_server_with_retry(&args.bind, args.max_bind_retries)?;

    let cache_duration = Duration::from_secs(args.cache_seconds);
    serve_requests(server, &args.kopia_bin, cache_duration);

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
    }

    #[test]
    fn test_start_server_with_retry_exhausted() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();

        let result = start_server_with_retry(&addr.to_string(), 2);
        assert!(result.is_err());
        let err_msg = result.err().unwrap().to_string();
        assert!(err_msg.contains("Failed to bind to"));
        assert!(err_msg.contains("after 2 attempts"));
    }
}
