//! `OutboundCallFixture` is a faithful `OutboundCall`: headers are added after the ones the call carries, a bad header is
//! refused with `InvalidArgument`, and the timeout can be set.

use std::time::Duration;

use grpc_egress_test_support::OutboundCallFixture;
use grpc_egress_transport_api::OutboundCall;
use proto_pattern::{CallError, Metadata, MetadataEntry, StatusCode};

/// @covers: OutboundCallFixture::add_header
#[test]
fn test_add_header_keeps_the_existing_headers_and_appends_the_new_one(
) -> Result<(), Box<dyn std::error::Error>> {
    let mut call = OutboundCallFixture::create("/svc/M")
        .with_metadata(Metadata::create(vec![MetadataEntry::create("x-a", "1")?]));

    call.add_header("x-a", "2")?;
    call.add_header("x-b", "3")?;

    assert_eq!(call.header_values("x-a"), ["1", "2"]);
    assert_eq!(call.header_values("x-b"), ["3"]);
    Ok(())
}

/// @covers: OutboundCallFixture::add_header
#[test]
fn test_add_header_with_a_reserved_key_is_refused_with_invalid_argument() {
    let mut call = OutboundCallFixture::create("/svc/M");

    let error = call.add_header("grpc-reserved", "x").err();

    assert!(
        matches!(
            error,
            Some(CallError::Transport {
                code: StatusCode::InvalidArgument,
                ..
            })
        ),
        "got {error:?}"
    );
    assert!(!call.has_header("grpc-reserved"));
}

/// @covers: OutboundCallFixture::set_timeout
#[test]
fn test_set_timeout_replaces_the_timeout_of_the_call() {
    let mut call = OutboundCallFixture::create("/svc/M").with_timeout(Some(Duration::from_secs(1)));

    call.set_timeout(Some(Duration::from_secs(9)));

    assert_eq!(call.timeout(), Some(Duration::from_secs(9)));
    assert_eq!(call.path(), "/svc/M");
}
