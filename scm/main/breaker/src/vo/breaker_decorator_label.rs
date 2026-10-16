//! `BreakerDecorator`-adjacent constant — the flat api/ counterpart to the
//! flat `core::breaker::breaker_decorator` file (see filename_matches_type /
//! core_api_module_correspondence pairing convention).

/// Human-readable label for the decorator this crate installs.
pub const BREAKER_DECORATOR_LABEL: &str = "grpc-breaker-decorator";
