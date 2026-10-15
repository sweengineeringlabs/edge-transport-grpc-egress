#[cfg(test)]
mod tests {
    use std::time::Duration;

    use crate::api::{BreakerState, Error, GrpcBreakerClient, GrpcBreakerConfig};

    #[test]
    fn test_new_starts_closed_with_supplied_config() {
        let rt = tokio::runtime::Runtime::new().expect("runtime");
        rt.block_on(async {
            let client = GrpcBreakerClient::new(
                (),
                GrpcBreakerConfig {
                    failure_threshold: 9,
                    cool_down_seconds: 5,
                    half_open_probe_count: 1,
                },
            );
            assert!(matches!(client.state().await, BreakerState::Closed));
        });
    }

    #[test]
    fn test_config_returns_the_supplied_policy() {
        let client = GrpcBreakerClient::new(
            (),
            GrpcBreakerConfig {
                failure_threshold: 12,
                cool_down_seconds: 3,
                half_open_probe_count: 2,
            },
        );
        assert_eq!(client.config().failure_threshold, 12);
    }

    #[test]
    fn test_state_reflects_closed_initial_state() {
        let rt = tokio::runtime::Runtime::new().expect("runtime");
        rt.block_on(async {
            let client = GrpcBreakerClient::new((), GrpcBreakerConfig::default());
            assert!(matches!(client.state().await, BreakerState::Closed));
        });
    }

    #[test]
    fn test_cool_down_converts_seconds_to_duration() {
        let cfg = GrpcBreakerConfig {
            failure_threshold: 1,
            cool_down_seconds: 17,
            half_open_probe_count: 1,
        };
        assert_eq!(cfg.cool_down(), Duration::from_secs(17));
    }

    #[test]
    fn test_from_config_parses_valid_toml() {
        let cfg = GrpcBreakerConfig::from_config(
            "failure_threshold = 3\ncool_down_seconds = 10\nhalf_open_probe_count = 1\n",
        )
        .expect("valid toml must parse");
        assert_eq!(cfg.failure_threshold, 3);
    }

    #[test]
    fn test_from_config_rejects_malformed_toml() {
        let err = GrpcBreakerConfig::from_config("not valid toml {{{")
            .expect_err("malformed toml must be rejected");
        assert!(matches!(err, Error::ParseFailed(_)));
    }
}
