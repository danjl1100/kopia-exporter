Currently, if the `kopia` subprocess (launched in src/lib.rs `fn new_from_command`) writes too much data to stdout, then it pauses until it is terminated. Testing showed it freezes after ~58KB is read, at which point the timeout terminates it correctly, but no metrics are served.

- [x] add a test case that triggers `fake_kopia` to output ~1MB of JSON and verifies the metrics page is served successfully
    - focus on testing the success, not the actual contents of the huge JSON.  Maybe entries at the beginning and end of the JSON with special names, and verify the name is present in `/metrics` with count of 2? (whichever metric is easiest to test that way)
- [x] implement the fix (thinking to stream stdout to the JSON deserializer, but any other better fixes are welcome)

Review comments:
- [x] in the new code added `fake_kopia`, panic if the input JSON doesn't contain the expected data.  Right now, if the input changes, the new test could fail, but it's really `fake_kopia`'s fault
