Currently, if the `kopia` command times out then there is no printout of stdout and stderr from that child process.

- [x] add a new test or update an existing test to verify stdout and stderr are printed out, using `src/bin/fake_kopia.rs` to provide the observable stdout and stderr strings
- [x] implement the stdout/stderr print logic so the new/updated test passes
