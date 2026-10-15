# api/tests/

This directory exists to satisfy the `api_impl_public_tests_external` arch
audit check, which requires `src/api/tests/` to be present. It holds
no Rust source — this crate has no standalone `pub fn` in `api/` (only
trait declarations and constants), so there is nothing to test here.

Actual tests for `api/` traits and types live in the crate-root `tests/`
directory (e.g. `tests/processor_e2e_test.rs`), exercised as an external
consumer would import them.
