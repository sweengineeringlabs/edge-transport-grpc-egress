//! Integration tests for `api/value/compression_mode.rs`.

use edge_transport_grpc_egress_transport::CompressionMode;

#[test]
fn transport_struct_default_is_none_compression_int_test() {
    assert_eq!(CompressionMode::default(), CompressionMode::None);
}

#[test]
fn transport_struct_header_value_for_none_returns_none_int_test() {
    assert_eq!(CompressionMode::None.header_value(), None);
}

#[test]
fn transport_struct_header_value_for_gzip_and_zstd_uses_canonical_names_int_test() {
    assert_eq!(CompressionMode::Gzip.header_value(), Some("gzip"));
    assert_eq!(CompressionMode::Zstd.header_value(), Some("zstd"));
}
