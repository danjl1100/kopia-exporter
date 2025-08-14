## project direction
- [x] Proofread the README.md. Identify any changes up-front to help improve overall clarity.
- [x] Review the goals stated in README.md, find any that may not be helpful for the user, or any others that are missing.
- [x] Define metrics following prometheus conventions that align with the goals. Identify any commonly-used backup metrics that are not mentioned in the goals. Check back with me to discuss if there's a compelling reason to include more than stated by the goals.

## `fake-kopia`
- [x] create a binary in `src/bin` as a stand-in for `kopia` (as it is not in the path during development)
- [x] write a parser for the sample input file `src/sample_kopia-snapshot-list.json`
- [x] determine useful metrics to expose to accomplish the goal stated in README.md

## update parse tests
- [ ] move the test in main.rs to a library test function in kopia.rs
- [ ] add a Command subprocess function to the kopia.rs module, to call the specified kopia bin and parse the output
- [ ] add another library test function to test the subprocess, using the fake-kopia bin for the input 
- [ ] add (using `cargo add`) the eyre crate and update all error handling to use that (with `?` operator where appropriate to reduce indentation drift)


## web server
- [ ] suggest a few options for a web server crate to use, focus on lightweight (small dependency count) for a tiny server
- [ ] implement the easiest 2 Prometheus metrics using the chosen web server crate in a new library module (sibling to kopia.rs)
- [ ] add an integration test using fake-kopia as the data source, to test the Command subprocess I/O as well