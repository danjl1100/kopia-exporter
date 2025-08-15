## cleanup
- [x] update the "/" web server endpoint to include a clickable link to "/metics" (not just the text)
- [x] in `main.rs`, separate the serve loop into a separate function, to show that it never returns an error
- [x] use `eyre` to report errors in the integration test, intead of expect/unwrap

## web server
- [x] suggest a few options for a web server crate to use, focus on lightweight (small dependency count) for a tiny server
- [x] implement the easiest 2 Prometheus metrics using the chosen web server crate in a new library module (sibling to kopia.rs)
- [x] add an integration test using fake-kopia as the data source, to test the Command subprocess I/O as well

## strengthen tests
- [ ] update one or more test cases to verify that `get_retention_counts` works correctly when a count occurs more than once.  -- actually, is it part of Kopia's design to tag more than one snapshot with the same retention label? Maybe this should aggregate all `monthly-*` flags to count as one `monthly` group, and so on for daily, weekly, etc.
