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
}

#[derive(Debug, Clone)]
struct CachedSnapshots {
    snapshots: Vec<Snapshot>,
    cached_at: Instant,
}

#[allow(clippy::needless_pass_by_value)] // Server is consumed by incoming_requests()
fn serve_requests(server: Server, kopia_bin: &str, cache_duration: Duration) {
    let mut cache: Option<CachedSnapshots> = None;
    for request in server.incoming_requests() {
        match (request.method(), request.url()) {
            (&Method::Get, "/metrics") => {
                // 1. Check if cached value is available (clear if expired)
                if !cache_duration.is_zero()
                    && let Some(ref cached) = cache
                    && cached.cached_at.elapsed() >= cache_duration
                {
                    cache = None; // Clear expired cache
                }

                // 2. Get snapshots (from cache or fresh fetch)
                let snapshots = if let Some(ref cached) = cache {
                    Ok(cached.snapshots.clone())
                } else {
                    match get_snapshots_from_command(kopia_bin) {
                        Ok(fresh_snapshots) => {
                            // Update cache if caching is enabled
                            if !cache_duration.is_zero() {
                                cache = Some(CachedSnapshots {
                                    snapshots: fresh_snapshots.clone(),
                                    cached_at: Instant::now(),
                                });
                            }
                            Ok(fresh_snapshots)
                        }
                        Err(e) => Err(e),
                    }
                };

                // 3. Serve the result
                match snapshots {
                    Ok(snapshots) => {
                        let metrics_output = metrics::generate_all_metrics(&snapshots);
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

fn main() -> eyre::Result<()> {
    let args = Args::parse();

    println!("Starting Kopia Exporter on {}", args.bind);

    let server =
        Server::http(&args.bind).map_err(|e| eyre::eyre!("Failed to start server: {}", e))?;

    let cache_duration = Duration::from_secs(args.cache_seconds);
    serve_requests(server, &args.kopia_bin, cache_duration);

    Ok(())
}
