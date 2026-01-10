//! Single test binary to allow integration to run in parallel

const FAKE_KOPIA_BIN: &str = env!("CARGO_BIN_EXE_fake-kopia");

mod common {
    mod bind_retry_test;
    mod integration_test;
}

mod test_helpers;
