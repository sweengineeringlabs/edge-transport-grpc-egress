//! Integration tests for the interceptor contract: the trait is object-safe, and `OutboundCallHeaders` reads and adds
//! the request headers of an `OutboundCall` the way an interceptor needs.

use grpc_egress_test_support::OutboundCallFixture;
use grpc_egress_transport_api::{GrpcEgressInterceptor, OutboundCall};
use proto_pattern::{
    BinaryMetadataEntry, CallError, GetBinaryEntriesRequest, GetCountRequest, Metadata,
    MetadataEntry, MetadataView, StatusCode,
};

#[test]
fn test_interceptor_trait_is_object_safe() {
    fn _assert(_: &dyn GrpcEgressInterceptor) {}
}

fn call_with(metadata: Metadata) -> OutboundCallFixture {
    OutboundCallFixture::create("/svc/Method")
        .with_timeout(None)
        .with_metadata(metadata)
}

fn entry(key: &str, value: &str) -> Result<MetadataEntry, Box<dyn std::error::Error>> {
    Ok(MetadataEntry::create(key, value)?)
}

/// @covers: OutboundCallHeaders::has_header
#[test]
fn test_has_header_call_without_the_header_returns_false() {
    assert!(!call_with(Metadata::default()).has_header("x-token"));
}

/// @covers: OutboundCallHeaders::has_header
#[test]
fn test_has_header_call_that_carries_the_header_returns_true(
) -> Result<(), Box<dyn std::error::Error>> {
    let call = call_with(Metadata::create(vec![entry("x-token", "t")?]));
    assert!(call.has_header("x-token"));
    assert!(
        !call.has_header("x-other"),
        "a header of another name is not this header"
    );
    Ok(())
}

/// @covers: OutboundCallHeaders::add_header
#[test]
fn test_add_header_adds_after_the_headers_the_call_carries(
) -> Result<(), Box<dyn std::error::Error>> {
    let mut call = call_with(Metadata::create(vec![entry("x-caller", "original")?]));

    call.add_header("x-added", "1")?;

    assert_eq!(call.header_values("x-caller"), ["original"]);
    assert_eq!(call.header_values("x-added"), ["1"]);
    assert_eq!(
        call.get_metadata()
            .get_count(GetCountRequest::new())?
            .into_count(),
        2
    );
    Ok(())
}

/// @covers: OutboundCallHeaders::add_header
#[test]
fn test_add_header_same_key_twice_keeps_both_values_in_order() -> Result<(), CallError> {
    let mut call = call_with(Metadata::default());

    call.add_header("x-multi", "first")?;
    call.add_header("x-multi", "second")?;

    assert_eq!(call.header_values("x-multi"), ["first", "second"]);
    Ok(())
}

/// @covers: OutboundCallHeaders::add_header
#[test]
fn test_add_header_keeps_the_binary_entries_the_call_carries(
) -> Result<(), Box<dyn std::error::Error>> {
    let blob = BinaryMetadataEntry::create("x-blob-bin", vec![1u8, 2, 3])?;
    let mut call = call_with(Metadata::create_with_binary(
        vec![entry("x-caller", "original")?],
        vec![blob],
    ));

    call.add_header("x-added", "1")?;

    let kept: Vec<(String, Vec<u8>)> =
        MetadataView::get_binary_entries(call.get_metadata(), GetBinaryEntriesRequest::new())?
            .into_entries()
            .flatten()
            .map(|b| (b.get_key().to_string(), b.get_value().to_vec()))
            .collect();
    assert_eq!(kept, [("x-blob-bin".to_string(), vec![1, 2, 3])]);
    Ok(())
}

/// @covers: OutboundCallHeaders::add_header
#[test]
fn test_add_header_invalid_key_returns_invalid_argument_naming_the_path_and_changes_nothing(
) -> Result<(), Box<dyn std::error::Error>> {
    let mut call = call_with(Metadata::create(vec![entry("x-caller", "original")?]));

    let error = call
        .add_header("Bad Key", "v")
        .err()
        .ok_or("a key with a capital and a space must be refused")?;

    assert!(
        matches!(
            &error,
            CallError::Transport { path, code: StatusCode::InvalidArgument, .. }
                if path.as_ref() == "/svc/Method"
        ),
        "got {error:?}"
    );
    assert_eq!(
        call.get_metadata()
            .get_count(GetCountRequest::new())?
            .into_count(),
        1,
        "a refused header is not added"
    );
    Ok(())
}

/// @covers: OutboundCallHeaders::add_header
#[test]
fn test_add_header_value_with_a_line_break_returns_invalid_argument() {
    let mut call = call_with(Metadata::default());

    let result = call.add_header("x-token", "line\nbreak");

    assert!(
        matches!(
            result,
            Err(CallError::Transport {
                code: StatusCode::InvalidArgument,
                ..
            })
        ),
        "got {result:?}"
    );
}
