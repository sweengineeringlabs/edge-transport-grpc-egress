//! Integration tests for `api/port/grpc/grpc_channel_config_error.rs`.

use grpc_egress_transport_api::GrpcChannelConfigError;

#[test]
fn transport_struct_plaintext_rejected_display_includes_endpoint_int_test() {
    let e = GrpcChannelConfigError::PlaintextRejected("http://x".into());
    assert!(e.to_string().contains("http://x"));
}

#[test]
fn transport_struct_unusable_tls_file_display_names_the_path_and_the_reason_int_test() {
    let e = GrpcChannelConfigError::UnusableTlsFile {
        path: "/etc/tls/client.pem".into(),
        reason: "no certificate found in the file".into(),
    };
    let text = e.to_string();
    assert!(text.contains("/etc/tls/client.pem"), "{text}");
    assert!(text.contains("no certificate found"), "{text}");
}

#[test]
fn transport_struct_tls_option_on_plaintext_endpoint_display_names_option_and_endpoint_int_test() {
    let e = GrpcChannelConfigError::TlsOptionOnPlaintextEndpoint {
        option: "mtls",
        endpoint: "http://x".into(),
    };
    let text = e.to_string();
    assert!(text.contains("mtls") && text.contains("http://x"), "{text}");
}
