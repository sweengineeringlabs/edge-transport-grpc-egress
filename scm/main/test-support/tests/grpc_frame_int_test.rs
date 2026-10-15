//! Tests of `GrpcFrame`: the five-byte prefix, the compressed flag, and partial input.

use grpc_egress_test_support::{DecodedFrame, GrpcFrame};

/// @covers: GrpcFrame::encode
#[test]
fn test_encode_payload_writes_flag_zero_and_big_endian_length() {
    let framed = GrpcFrame::encode(b"hello");

    assert_eq!(
        &framed[..],
        &[0x00, 0, 0, 0, 5, b'h', b'e', b'l', b'l', b'o']
    );
}

/// @covers: GrpcFrame::encode_compressed
#[test]
fn test_encode_compressed_payload_sets_the_flag_byte() {
    let framed = GrpcFrame::encode_compressed(b"xyz");

    assert_eq!(framed[0], 0x01);
    assert_eq!(&framed[1..5], &[0, 0, 0, 3]);
}

/// @covers: GrpcFrame::decode_all
#[test]
fn test_decode_all_two_messages_returns_both_in_order() {
    let mut body = GrpcFrame::encode(b"one").to_vec();
    body.extend_from_slice(&GrpcFrame::encode_compressed(b"two"));

    let frames = GrpcFrame::decode_all(&body);

    assert_eq!(
        frames,
        vec![
            DecodedFrame::create(false, b"one".to_vec()),
            DecodedFrame::create(true, b"two".to_vec()),
        ]
    );
}

/// @covers: GrpcFrame::decode_all
#[test]
fn test_decode_all_truncated_message_is_ignored_not_returned_short() {
    let framed = GrpcFrame::encode(b"complete");
    let truncated = &framed[..framed.len() - 1];

    assert!(GrpcFrame::decode_all(truncated).is_empty());
}

/// @covers: GrpcFrame::decode_all
#[test]
fn test_decode_all_empty_input_returns_no_messages() {
    assert!(GrpcFrame::decode_all(&[]).is_empty());
}

/// @covers: GrpcFrame::decode_all
#[test]
fn test_decode_all_empty_payload_returns_one_empty_message() {
    let frames = GrpcFrame::decode_all(&GrpcFrame::encode(b""));

    assert_eq!(frames, vec![DecodedFrame::create(false, Vec::new())]);
}
