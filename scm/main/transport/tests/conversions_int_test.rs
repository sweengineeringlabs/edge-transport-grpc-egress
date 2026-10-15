//! Integration tests for `api/status/conversions.rs`.

use edge_transport_grpc_egress_transport::Conversions;

/// @covers: Conversions is a zero-sized marker — the interface declaration
/// counterpart for core/status/conversions.rs, not itself carrying state.
#[test]
fn transport_struct_conversions_marker_is_constructable_int_test() {
    let _ = Conversions;
    assert_eq!(
        std::mem::size_of::<Conversions>(),
        0,
        "Conversions is a marker type and must carry no instance state"
    );
}
