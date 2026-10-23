//! Coverage stub for `src/vo/grpc_retry_config.rs`.

use grpc_egress_retry_api::GrpcRetryConfig;
use grpc_egress_retry_api::GrpcRetryConfigProvider as _;

/// @covers: GrpcRetryConfig — type is accessible and holds real fields (not zero-sized)
#[test]
fn retry_struct_grpc_retry_config_is_accessible_int_test() {
    assert!(
        std::mem::size_of::<GrpcRetryConfig>() > 0,
        "GrpcRetryConfig holds max_attempts/backoff/jitter fields and must not be zero-sized"
    );
}

/// @covers: GrpcRetryConfig::default — SWE default values are valid
#[test]
fn retry_struct_grpc_retry_config_default_passes_validation_int_test() {
    let cfg = GrpcRetryConfig::default();
    assert!(cfg.max_attempts() >= 1);
    assert!(cfg.backoff_multiplier() > 0.0);
    assert!((0.0..=1.0).contains(&cfg.jitter_factor()));
    assert!(cfg.max_backoff_ms() >= cfg.initial_backoff_ms());
}

/// @covers: GrpcRetryConfig::from_config — parses TOML correctly
#[test]
fn retry_struct_grpc_retry_config_from_config_parses_full_toml_int_test(
) -> Result<(), Box<dyn std::error::Error>> {
    let toml = r#"
        max_attempts = 3
        initial_backoff_ms = 50
        backoff_multiplier = 2.0
        jitter_factor = 0.1
        max_backoff_ms = 1000
        rate_limit_max_attempts = 2
        rate_limit_initial_backoff_ms = 100
        rate_limit_max_backoff_ms = 5000
    "#;
    let cfg = retry_provider().parse_config(toml)?;
    assert_eq!(cfg.max_attempts(), 3);
    Ok(())
}

/// @covers: GrpcRetryConfigProvider::validate — a policy that allows no attempt is refused
#[test]
fn retry_struct_grpc_retry_config_validate_zero_attempts_is_refused_and_the_default_is_accepted_int_test(
) {
    let provider = retry_provider();
    let defaults = GrpcRetryConfig::default();
    let zero = GrpcRetryConfig::try_new(grpc_egress_retry_api::BuildGrpcRetryConfigRequest {
        max_attempts: 0,
        initial_backoff_ms: defaults.initial_backoff_ms(),
        backoff_multiplier: defaults.backoff_multiplier(),
        jitter_factor: defaults.jitter_factor(),
        max_backoff_ms: defaults.max_backoff_ms(),
        rate_limit_max_attempts: defaults.rate_limit_max_attempts(),
        rate_limit_initial_backoff_ms: defaults.rate_limit_initial_backoff_ms(),
        rate_limit_max_backoff_ms: defaults.rate_limit_max_backoff_ms(),
    });

    assert!(provider.validate(&GrpcRetryConfig::default()).is_ok());
    assert!(
        matches!(zero, Err(grpc_egress_retry_api::Error::InvalidConfig(ref m)) if m.contains("max_attempts"))
    );
}

/// @covers: GrpcRetryConfigProvider::parse_config — a policy with zero attempts is refused as text too
#[test]
fn retry_struct_grpc_retry_config_parse_config_zero_attempts_is_refused_int_test() {
    let toml = r#"
        max_attempts = 0
        initial_backoff_ms = 50
        backoff_multiplier = 2.0
        jitter_factor = 0.1
        max_backoff_ms = 1000
        rate_limit_max_attempts = 2
        rate_limit_initial_backoff_ms = 100
        rate_limit_max_backoff_ms = 5000
    "#;
    assert!(retry_provider().parse_config(toml).is_err());
}

/// @covers: GrpcRetryConfigProvider::initial_backoff — milliseconds
#[test]
fn retry_struct_grpc_retry_config_initial_backoff_is_read_in_milliseconds_int_test(
) -> Result<(), grpc_egress_retry_api::Error> {
    let provider = retry_provider();
    let config = GrpcRetryConfig::try_new(grpc_egress_retry_api::BuildGrpcRetryConfigRequest {
        max_attempts: 5,
        initial_backoff_ms: 250,
        backoff_multiplier: 2.0,
        jitter_factor: 0.1,
        max_backoff_ms: 7_000,
        rate_limit_max_attempts: 2,
        rate_limit_initial_backoff_ms: 1_500,
        rate_limit_max_backoff_ms: 9_000,
    })?;
    assert_eq!(
        provider.initial_backoff(&config),
        std::time::Duration::from_millis(250)
    );
    assert_eq!(
        provider.max_backoff(&config),
        std::time::Duration::from_millis(7_000)
    );
    assert_eq!(
        provider.rate_limit_initial_backoff(&config),
        std::time::Duration::from_millis(1_500)
    );
    assert_eq!(
        provider.rate_limit_max_backoff(&config),
        std::time::Duration::from_millis(9_000)
    );
    Ok(())
}

fn retry_provider() -> impl grpc_egress_retry_api::GrpcRetryConfigProvider {
    use grpc_egress_retry_api::GrpcRetryConfigProviderFactory;
    grpc_egress_retry_spi::StdGrpcRetryConfigProviderFactory
        .build(grpc_egress_retry_api::BuildGrpcRetryConfigProviderRequest)
}
