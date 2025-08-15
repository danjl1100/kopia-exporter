//! Kopia Exporter - A Prometheus metrics exporter for Kopia backup repositories.
//!
//! This application exports metrics from Kopia backup repositories in a format
//! suitable for Prometheus monitoring.

use clap::Parser;
use kopia_exporter::{get_snapshots_from_command, metrics};
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
}

#[allow(clippy::needless_pass_by_value)] // Server is consumed by incoming_requests()
fn serve_requests(server: Server, kopia_bin: &str) {
    for request in server.incoming_requests() {
        match (request.method(), request.url()) {
            (&Method::Get, "/metrics") => match get_snapshots_from_command(kopia_bin) {
                Ok(snapshots) => {
                    let metrics_output = metrics::generate_all_metrics(&snapshots);
                    let header =
                        Header::from_bytes(&b"Content-Type"[..], &b"text/plain; charset=utf-8"[..])
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
            },
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

    serve_requests(server, &args.kopia_bin);

    Ok(())
}
