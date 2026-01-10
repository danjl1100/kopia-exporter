# kopia-exporter

See [the crate root docs](./src/lib.rs) for details on the motivation and metrics

## Development

This project is fully independent from any real `kopia` setup, so a fake kopia binary is included for testing to provide realistic test scenarios.

### Architecture

All core logic is implemented in the library crate (`src/lib.rs`), keeping the main binary lean and focused on CLI argument handling. This design allows for easy testing and potential future expansion (e.g., web server interface).
