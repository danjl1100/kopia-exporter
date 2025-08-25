- [x] update all dependencies to latest
- [x] look for patterns in the integration tests that can be extracted (to reduce repetition and make each test clearer), but think carefully to make sure the test effectiveness (what is being tested) remains high
- [x] similar as above, look for repetition across the nixos vm tests.  if any non-test related logic can be in a separate module, that would make it cleaner

## small cleanup items
- [ ] ServerConfig can store a `Command` directly, so the `with_args` and `with_env` functions can pass through to `Command` functions
- [ ] TestServer can store the bind address, so the get functions don't need the bind address argument
- [ ] update `get_test_port` to ask the OS for a random port (to avoid conflicts, e.g. if multiple `cargo test` invocations are running in parallel)
- [ ] the vm-test-lib.nix `httpTests` functions could use a `port` argument to avoid silent shared config
