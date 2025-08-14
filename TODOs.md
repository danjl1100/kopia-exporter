## update parse tests
- [x] move the test in main.rs to a library test function in kopia.rs
- [x] add a Command subprocess function to the kopia.rs module, to call the specified kopia bin and parse the output
- [x] add another library test function to test the subprocess, using the fake-kopia bin for the input
- [x] add (using `cargo add`) the eyre crate and update all error handling to use that (with `?` operator where appropriate to reduce indentation drift)


## web server
- [ ] suggest a few options for a web server crate to use, focus on lightweight (small dependency count) for a tiny server
- [ ] implement the easiest 2 Prometheus metrics using the chosen web server crate in a new library module (sibling to kopia.rs)
- [ ] add an integration test using fake-kopia as the data source, to test the Command subprocess I/O as well
