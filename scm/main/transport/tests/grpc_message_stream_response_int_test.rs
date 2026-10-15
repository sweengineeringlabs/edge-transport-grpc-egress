//! Integration tests for `GrpcMessageStreamResponse`.

use edge_transport_grpc_egress_transport::{GrpcEgressResult, GrpcMessageStreamResponse};
use futures::StreamExt as _;

/// @covers: GrpcMessageStreamResponse — can be constructed from an empty stream and
/// genuinely yields no items when polled, proving it's a real Stream, not a
/// value that merely type-checks against the alias.
#[tokio::test]
async fn transport_struct_grpc_message_stream_response_can_be_constructed_from_empty_stream_int_test(
) {
    use futures::stream;
    let mut s = GrpcMessageStreamResponse {
        stream: Box::pin(stream::empty::<GrpcEgressResult<Vec<u8>>>()),
    };
    assert!(
        s.stream.next().await.is_none(),
        "empty stream must yield no items"
    );
}
