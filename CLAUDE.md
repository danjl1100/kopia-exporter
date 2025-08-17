# kopia-exporter - Claude Code Workflow

## Project Overview
A lightweight Prometheus metrics exporter for Kopia backup repositories. Built with Rust, prioritizing minimal dependencies and comprehensive testing.

## Development Workflow

### Architecture Principles
- **Library-first**: Core logic in lib.rs, minimal main.rs
- **Framework agnostic**: Keep business logic separate from web framework
- **Lightweight dependencies**: Prefer minimal dependency count
- **Comprehensive testing**: Both unit tests and integration tests with real subprocess calls

### Key Dependencies
- **Web server**: `tiny_http` (only 5 additional dependencies)
- **HTTP client**: `minreq` (only 3 additional dependencies for dev/test)
- **Error handling**: `eyre` throughout
- **CLI**: `clap` with derive feature

### Testing Strategy
- **Unit tests**: Test individual functions and modules
- **Integration tests**: Test full subprocess pipeline with `fake-kopia` binary
- **Web server tests**: End-to-end HTTP testing with real server process
- **Test helpers**: Use helper functions to reduce verbose test setup

## File Organization
- `src/lib.rs`: Public library interface
- `src/kopia.rs`: Kopia data parsing and processing
- `src/metrics.rs`: Prometheus metrics generation
- `src/main.rs`: Web server and CLI interface
- `src/bin/fake-kopia.rs`: Test fixture for realistic testing
- `tests/`: Integration tests using real binaries
- `nixos-module/`: NixOS module and VM integration tests

## Testing

### Run All Tests
```bash
# Rust unit and integration tests
cargo test

# All checks including NixOS VM tests and formatting
nix flake check
```
