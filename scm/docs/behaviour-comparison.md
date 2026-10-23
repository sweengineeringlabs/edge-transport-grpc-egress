# Behaviour comparison: the hand-written `hyper` client against the `proto-svc` engine

Task 14 of `edge-transport-grpc-egress#18`. **Stage 1** is the engine swap (task 9): the `hyper` client replaced by
`proto-svc` behind the old `GrpcEgress` API (rows 1 to 17). **Stage 2** is the move of the API itself onto the
contract (ADR-009): rows 18 to 31. The method of stage 1: the 5.6k lines of tests of
`grpc-egress-transport-std`, written against the `hyper` client of v0.8.0, were run unchanged against the client over
`proto-svc` (`proto-pattern` 0.9.2, `proto-svc` 0.3.x). Every test that failed is a difference and is listed with
its decision; a test that passed is a behaviour that did not change. The first run failed 39 tests; the table
accounts for each. Nothing here was decided silently: a difference is **accepted** (the engine or the contract
decides, with its reason), **fixed** (restored in this crate) or **open** (not decided, named).

Method limits: the comparison covers what those tests observe. Not compared: metadata key case and non-ASCII
values, a response whose `content-type` is wrong, HTTP/2 connection reuse under load, and behaviour against any
server other than the in-process peers.

## Differences

| # | Case | Before (`hyper` client) | After (engine) | Decision |
|---|---|---|---|---|
| 1 | A call that passed its deadline (own deadline or request timeout) | `Timeout("request deadline exceeded")` | `Status(DeadlineExceeded, "the call did not complete within its timeout")` | **Accepted.** The contract reports a passed deadline as a status "whoever noticed it" (`proto-svc` `StatusAdapter`); task 6 of the issue names this mapping. Retry (`Terminal`) and breaker (not a failure) classify both alike; the load balancer now counts the status as a backend failure, as it counted `Timeout` |
| 2 | A response message over `max_message_bytes` | `Status(ResourceExhausted, "response body exceeded max_message_bytes")` | `Status(OutOfRange, "Error, decoded message length too large: ...")` | **Accepted, open to improve.** `OutOfRange` is `tonic`'s code, recorded by `proto-svc` as a difference from the gRPC table (which says `RESOURCE_EXHAUSTED`). The old client was the conformant one here. Restoring it in this crate means matching `tonic`'s message text, which is brittle; the proper fix is upstream. Retry and breaker treat both as terminal and not a breaker failure |
| 3 | A few hundred compressed bytes that expand past the cap | `Status(ResourceExhausted)` | `Status(ResourceExhausted)` | **Unchanged.** The decompression limit holds (test passed without edits) |
| 4 | A response compressed with an encoding the client did not advertise | decoded | `Status(Unimplemented, "Content is compressed with ... which isn't supported")` | **Accepted.** The protocol lets a server use only an encoding the client listed in `grpc-accept-encoding`; the engine accepts only those. `with_compression(Gzip or Zstd)` advertises and accepts it, and the tests that decode an answer now set it. A client that must read an unannounced encoding no longer can |
| 5 | A unary response with no message | an empty message | `Status(Unimplemented, "the peer ended the call without sending a message")` | **Accepted.** `proto-pattern` ADR-007 and the conformance table: a unary response with none or several messages is `UNIMPLEMENTED` at the client |
| 6 | A unary response with two messages | `Internal("internal client error")` | `Status(Unimplemented, ...)` | **Accepted** (same decision as 5) |
| 7 | A frame that ends before its announced length | `Internal("internal client error")` | `Status(Unimplemented, "the peer ended the call without sending a message")` | **Accepted.** `tonic` drops an unfinished final frame when the call ends with status OK, so the call has no message. The partial message is never delivered (test asserts the call fails) |
| 8 | Bytes after the last complete frame, fewer than a frame header | `Internal("internal client error")` | the message is delivered; the stray bytes are ignored | **Accepted.** `tonic` ignores them. The message before them is intact (test asserts it) |
| 9 | A frame with an invalid compressed flag; a compressed flag with no `grpc-encoding`; an invalid gzip body | `Internal("internal client error")` (sanitized) | `Status(Internal, "<tonic's text>")` | **Accepted.** The text names no address, path or internal type, and a status the peer sends was always passed through as it is; this crate cannot tell `tonic`'s own protocol errors from a peer's `INTERNAL`, so it cannot sanitize only the first |
| 10 | Streaming calls | the whole request stream and the whole response were buffered; the call failed at the call | the streams run as they arrive; a failure while reading is an item of the stream | **Improvement, behaviour change.** A server stream, client stream or bidirectional stream no longer holds a whole body in memory; a malformed frame surfaces while the stream is read (test reads the stream and asserts no partial message is delivered) |
| 11 | A client-streaming call whose peer replies with two messages | `Internal` | `Status(Unimplemented, ...)` | **Accepted** (as 5 and 6) |
| 12 | The provider of `rustls` | `aws-lc-rs`, installed process-wide by building a client (ADR-005) | `ring`, set by `tonic`'s `tls-ring` feature; nothing is installed process-wide | **Accepted.** ADR-005's aim (a TLS client never panics for want of a provider) holds, and the TLS tests prove a handshake works. The test of the old mechanism (`rustls_provider_int_test`) is removed with this reason |
| 13 | Trust when no CA is given (`https`) | the bundled web PKI roots (`webpki-roots`) | the platform's root store (`tls-native-roots`) | **Accepted, security-relevant.** Default trust now follows the operating system's store; `with_root_ca` still trusts that CA alone |
| 14 | The TLS handshake fails (a client with no certificate against a server that requires one) | `ConnectionFailed` | `Transport` with `Unknown`, which the mapper turns into `ConnectionFailed` | **Fixed.** `Unknown` and `Unavailable` transport failures are `ConnectionFailed`, as before |
| 15 | `with_prost_codec` (content type `application/grpc+proto`) | sent `application/grpc+proto` | the engine always sends `application/grpc`; the content subtype is outside the contract (`proto-pattern` ADR-007) | **Accepted: the option is removed.** Kept, it would be accepted and change nothing on the wire. `with_prost_codec`, `DisabledFeatureOptions`, the example's prost scenario and the tests that observed the content type are removed; the `prost` feature remains for the typed helper `ProstUnaryCaller`, which is a separate capability that moves to the contract's typed layer with the type replacement. Anyone who needs the content subtype needs it in the contract (`proto-pattern` ADR-007) |
| 16 | Load-balanced calls | a client copy retargeted to the picked backend on every call | one client per backend, built once; each keeps its connection | **Improvement.** Same pool and outcome reporting |
| 17 | A peer that sent `grpc-status` in the response headers together with a body | read as a normal answer | read as a "Trailers-Only" response with no message, so the size limit is not applied | **Fixed in the test peer.** That response is not what a server sends; `PeerServer` now sends the status in the trailers after the message. Real servers do, and `proto-svc` enforces the cap against them |

## Differences of stage 2: the API on the contract

| # | Case | Before | After | Decision |
|---|---|---|---|---|
| 18 | Metadata keys and values | any HTTP header name and value, one value per key (`HashMap<String, String>`) | the contract's rules: lowercase ASCII, digits, `-`, `_`, `.`; no `grpc-` prefix and no protocol header; a `-bin` key is a binary entry; values printable ASCII without leading or trailing space. Many values per key and binary entries survive a call | **Accepted, a fix and a tightening.** The refusal is made when the `MetadataEntry` is created, not at the call. Two values under one key and binary entries are proved against a real server; neither could be sent before |
| 19 | A path that is not `/service/method` | `Internal("internal client error")` at the call | refused when the `MethodPath` is created (`PathError`) | **Accepted** (contract, ADR-008 of `proto-pattern`) |
| 20 | Request size | only a response was capped (`max_message_bytes`) | a file's `max_message_bytes` sets the decoding **and** the encoding limit; a request over it never reaches the peer (`ResourceExhausted`) | **Accepted, a tightening.** The builder has the two limits as separate options |
| 21 | The request timeout | a backstop for a unary call and the streams that carried a deadline; a server stream used only the call's own deadline | the earlier of the call's own timeout and the request timeout, for every shape | **Accepted.** One rule for every shape; a long server stream needs a longer `with_request_timeout` or a stream-sized call timeout |
| 22 | `retry` and `breaker` and streaming calls | the three streaming shapes were not forwarded: `Unimplemented` | forwarded; retry does not retry a stream, the breaker admits the open and records its outcome | **Fixed** (a defect of the old clients) |
| 23 | Retry of a typed call | n/a (bytes only) | the original message is sent on the first attempt and rebuilt from its encoding for a later one | **Accepted.** A typed message cannot be cloned; with one attempt allowed nothing is encoded |
| 24 | Retry classification | by the message text of the error, `Timeout` and `Unavailable` variants | by `StatusCode`: transport `Unavailable`/`Unknown` and status `Unavailable` retry; `ResourceExhausted` by the message context, with the retry-after hint read from the trailers first | **Accepted.** The hint no longer has to be embedded in the message; the old embedding is still read |
| 25 | The breaker when open | `Unavailable(..)` | `Transport` with `Unavailable` (nothing was sent) | **Accepted** |
| 26 | `health_check` | opened a TCP connection and said nothing about the service | the `grpc.health.v1.Health/Check` call, tested against a real `tonic-health` server | **Improvement, behaviour change.** A caller that wants only reachability opens a connection itself |
| 27 | `call_unary_with_context` | the default method ignored the context entirely; no implementation used it | a view of the client that sends `x-trace-id`, `x-tenant-id`, `authorization: Bearer <token>` (authenticated only) and the context's metadata, never over a header the call already carries | **Improvement, behaviour change.** The capability now does something; an invalid context entry fails the call with `InvalidArgument` and nothing is sent |
| 28 | Cancellation | a `CancellationToken` in the request, raced against the call | dropping the call's future cancels it on the server (proved against a real server); a view bound to a caller-owned `CancellationSignal` adapter fails the call and each stream read with `Cancelled` | **Accepted** (ADR-008 ruling 2); the capability is kept, as a view bound to an interface |
| 29 | Interceptors | `before_call(&mut GrpcRequest)`, `after_call` could rewrite the response body | `before_call(&mut OutboundCall)`, `after_call(&InboundReply)` (headers and trailers, no body) | **Accepted.** The typed layer has no bytes to rewrite, and no interceptor of this repository rewrote a response |
| 30 | The bearer interceptor | overwrote an `authorization` header the call carried | keeps it, and mints nothing | **Accepted.** A header the caller set is the caller's |
| 31 | A load-balanced client | a copy of one client retargeted per call | one client per backend, built once (stage 1, row 16); `Transport(Unavailable)` when no backend is healthy | **Improvement** |

## What did not change

Interceptor order and error handling, trace context, retry-after hint embedding (now read from the trailers of the
status), compression on the way out (the frame is compressed and `grpc-encoding` announces it), keep-alive,
mutual TLS with client certificates, endpoint and option validation, load-balancer outcome judging, path
normalisation, the status codes a server sends (all 17 pass through), connection failures, cancellation by token.
Each is a test that passed unchanged.

## Consequences for a caller

- Code that matches `GrpcEgressError::Timeout` for a deadline must match `Status(DeadlineExceeded, _)`. The client
  no longer produces `Timeout`; the variant goes in the type-replacement stage.
- Code that matches `Status(ResourceExhausted, _)` for an oversize response must also match `Status(OutOfRange, _)`.
- Code that read an unannounced compressed response must set `with_compression`.
- `rust-version` is 1.99 (the engine requires it).
