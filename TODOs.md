- [x] add optional basic authentication, controllable via CLI args and the nixos module options

## cleanup
- [x] remove the `use std::str;` in main.rs, it's already imported by default in the language prelude
- [x] add an assert in the nixos module to ensure either (auth.username and auth.password) xor (credentialsFile)  is used
- [x] add incorrect credentials case to `test_basic_auth_credentials_file_integration`

## later (do not start these yet)
- [ ] update all dependencies to latest
- [ ] look for patterns in the integration tests that can be extracted (to reduce repetition and make each test clearer), but think carefully to make sure the test effectiveness (what is being tested) remains high
- [ ] similar as above, look for repetition across the nixos vm tests.  if any non-test related logic can be in a separate module, that would make it cleaner
