## cleanup
- [x] update the "/" web server endpoint to include a clickable link to "/metics" (not just the text)
- [x] in `main.rs`, separate the serve loop into a separate function, to show that it never returns an error
- [x] use `eyre` to report errors in the integration test, intead of expect/unwrap
- [x] for all test (library tests and integration tests) try to simplify the test by using a helper function to create snapshots. goal is to make it easier to tell at a glance which snapshot fields are relevant to the specific test.  It's possible that different tests could need different helpers.

## web server
- [x] suggest a few options for a web server crate to use, focus on lightweight (small dependency count) for a tiny server
- [x] implement the easiest 2 Prometheus metrics using the chosen web server crate in a new library module (sibling to kopia.rs)
- [x] add an integration test using fake-kopia as the data source, to test the Command subprocess I/O as well

## strengthen tests
- [x] update one or more test cases to verify that `get_retention_counts` works correctly when a count occurs more than once.  -- actually, is it part of Kopia's design to tag more than one snapshot with the same retention label? Maybe this should aggregate all `monthly-*` flags to count as one `monthly` group, and so on for daily, weekly, etc.

**ANALYSIS RESULT**: Current design is correct. Kopia uses numbered retention slots (monthly-1, monthly-2, etc.) representing specific positions in the retention timeline, not counts. Each slot should be tracked separately for operational monitoring (gap detection, retention policy verification, debugging). Added test confirms this behavior.
