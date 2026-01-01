The integration tests use a test binary `fake-kopia` in order to test the `kopia` invocations in a controlled manner.

ISSUE: If `kopia` hangs indefinintely (e.g. if the network is down, and not immediately reported as an error) then the web server also hangs.  For the purpose of debugging, this server should reply with HTTP `500` to indicate the intermittent timeout issue.

WORK DONE: Already added a new env var `FAKE_KOPIA_SLEEP_FOR_SECS` to the `fake-kopia.rs` test double.

TODO:
- [ ] add a new test in `tests/integration_test.rs`:
    1. SET trigger the new `FAKE_KOPIA_SLEEP_FOR_SECS` env var with value `1` in the test binary `src/bin/fake-kopia.rs`
    2. SET `--timeout 0.5` (to shorten the timeout to 1/2 second, for the test)
    3. VERIFY the `FAKE_KOPIA_LOG` log reports the input sleep (e.g. to avoid false-pass based on incorrectly passed sleep parameters)
    4. VERIFY the metric endpoint responds with HTTP status code `500`
- [ ] refactor the new test into a function to avoid duplication when adding another test that gives a string `"forever"` for the env var, verifying the same result (timing out after 0.5 seconds with `500` HTTP error code)
- [ ] update the library to timeout when waiting on output from the kopia subcommand
    - default timeout = 15 seconds, can be tunable by CLI argument `--timeout` (`f64`) to main
    - no added dependencies - prefer a manual implementation using spawn() and thread timing
    - update `fn get_snapshots_from_command` signature to add argument `timeout: Duration` (`std::time::Duration`)
    - include "timeout" in the error message for easier debugging
