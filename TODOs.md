## cleanup
- [ ] address the couple of `TODO` comments present in the code
- [ ] fix `cargo clippy` lints, including documenting the pub library functions

## web server
- [ ] suggest a few options for a web server crate to use, focus on lightweight (small dependency count) for a tiny server
- [ ] implement the easiest 2 Prometheus metrics using the chosen web server crate in a new library module (sibling to kopia.rs)
- [ ] add an integration test using fake-kopia as the data source, to test the Command subprocess I/O as well
