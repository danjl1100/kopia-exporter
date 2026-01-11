A recent change added `{source="..."}` to the metrics output, but the tests only verify the output for one `source`.
To guard against future regressions, the multi-source needs test coverage.

- [x] add tests for all metrics (where applicable) to verify the output format distinguishes multiple sources (`user_name@host:/path` combos), reporting the correct metric value for each source
    - avoid large blocks of JSON text for the inputs, instead prefer to use a helper function to create a snapshot, then edit the fields like `src/metrics/last_timestamp.rs`
