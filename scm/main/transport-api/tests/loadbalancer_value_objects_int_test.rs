//! Construction and deserialization invariants for load-balancer value objects.

use grpc_egress_transport_api::{BackendTarget, BalancingStrategy, LoadbalancerSettings};

#[test]
fn test_backend_target_rejects_empty_uri_and_zero_weight() {
    assert!(BackendTarget::try_new("", 1).is_err());
    assert!(BackendTarget::try_new("https://service", 0).is_err());
}

#[test]
fn test_loadbalancer_settings_requires_at_least_one_backend() {
    assert!(LoadbalancerSettings::try_new(BalancingStrategy::RoundRobin, Vec::new()).is_err());
}

#[test]
fn test_deserialization_rejects_invalid_backend_values() {
    let empty_uri = r#"strategy = "round-robin"
[[backends]]
url = ""
weight = 1
"#;
    let zero_weight = r#"strategy = "weighted"
[[backends]]
url = "https://service"
weight = 0
"#;
    assert!(toml::from_str::<LoadbalancerSettings>(empty_uri).is_err());
    assert!(toml::from_str::<LoadbalancerSettings>(zero_weight).is_err());
    assert!(
        toml::from_str::<LoadbalancerSettings>("strategy = \"round-robin\"\nbackends = []\n")
            .is_err()
    );
}
