# Proposal for `proto-pattern`: a path-addressed byte call (`RawTransport`)

**Status:** Draft for review. Nothing in `proto-pattern` or `proto-svc` has been changed.
**Date:** 2026-10-16
**Author:** Software Engineering Labs
**Needed by:** `edge-transport-grpc-egress#18` (ADR-008). If this is accepted, the byte-level `GrpcEgress` of that repository can be replaced instead of kept beside the contract.
**Targets:** `proto-pattern` 0.9.0 (additive), then `proto-svc` 0.3.0.

---

## 1. Problem

`Transport::call::<M>` takes the path of the call from `M::PATH`, an associated constant of a type. A caller that holds the path only as a string at run time (a router that forwards a call it did not define, a generic client, a test tool, a client generated from configuration) cannot make the call: it would need one `Method` type per path, and a type cannot be created at run time. Such a caller has to leave the contract and use its own, which is the duplication `edge-transport-grpc-egress#18` wants to remove.

The old surface that has this shape is `GrpcEgress::call_unary(GrpcRequest { method: String, body: Vec<u8>, .. })` and its streaming siblings.

## 2. Constraints taken from the accepted ADRs of `proto-pattern`

- Zero dependency, no build script (ADR-001).
- Static dispatch: no `dyn`, no `Box`, no `async_trait` in any trait (ADR-001).
- Factories return opaque `impl` types (ADR-002).
- Values are encapsulated: private fields, `create` constructors, `get_*` accessors (README, guarantees).
- Errors keep the meaning of the protocol document (ADR-006, ADR-007).
- Additive: a `Transport` implementer, in `proto-svc` or in a consumer's test, must keep compiling.

## 3. Options

1. **A second trait beside `Transport`** (recommended). `RawTransport` makes the four call shapes for a path given as a value, with bytes for messages. A svc implements both traits over one engine.
2. **Make `Method` a value** (`fn path(&self) -> &str`) and add `Transport::call_with(&method, ..)`. One trait, but the associated type `M::Request` of every existing signature changes meaning, and every implementer breaks. Rejected: not additive.
3. **A macro that creates a `Method` type per path.** Needs the path at compile time, which is the thing missing. Rejected.
4. **Leave it out.** The consumers that need a run-time path keep a contract of their own, beside this one. Rejected: it is the duplication the move exists to end.

## 4. Design of option 1

New value, in `vo/`:

```rust
/// A path of a call: `/package.Service/Method`.
pub struct MethodPath { path: String }
impl MethodPath {
    /// Fails with `PathError` unless the text is `/` + a service name + `/` + a method name,
    /// each non-empty, with no further `/` and no whitespace or control character.
    pub fn create(path: impl Into<String>) -> Result<Self, PathError>;
    pub fn get_path(&self) -> &str;
}
```

New error, in `error/`: `PathError` (the path is empty, lacks the leading `/`, has not exactly two segments, or holds a character a request line cannot carry). The check lives in the contract, as the metadata rule does (ADR-003 item 2), so every transport refuses the same paths and a malformed path never reaches the wire.

New trait, in `traits/`:

```rust
pub trait RawTransport: Send + Sync {
    type Metadata: MetadataView + Send + Sync + 'static;

    fn call_raw(&self, path: &MethodPath, request: CallRequest<Vec<u8>>)
        -> impl Future<Output = Result<CallResponse<Vec<u8>, Self::Metadata>, CallError>> + Send;

    fn call_raw_server_stream(&self, path: &MethodPath, request: CallRequest<Vec<u8>>)
        -> impl Future<Output = Result<impl ResponseStream<Vec<u8>, Metadata = Self::Metadata>, CallError>> + Send;

    fn call_raw_client_stream<S: MessageStream<Vec<u8>> + 'static>(&self, path: &MethodPath, requests: S, options: CallOptions)
        -> impl Future<Output = Result<CallResponse<Vec<u8>, Self::Metadata>, CallError>> + Send;

    fn call_raw_bidi_stream<S: MessageStream<Vec<u8>> + 'static>(&self, path: &MethodPath, requests: S, options: CallOptions)
        -> impl Future<Output = Result<impl ResponseStream<Vec<u8>, Metadata = Self::Metadata>, CallError>> + Send;
}
```

Everything else already exists and is reused: `CallRequest`, `CallOptions`, `CallResponse`, `MessageStream`, `ResponseStream`, `Metadata`, `CallError`, `StatusCode`.

### Meaning of the errors

The same as for a typed call, with two differences that follow from the bytes: `CallError::Encode` and `CallError::Decode` never occur (the message is the bytes the caller gave and the bytes the peer sent), and the size limits, the deadline, the status of a reply without `grpc-status`, and the cardinality of a unary reply (none or several is `Unimplemented`) are exactly those of `Transport::call`. A path that fails `MethodPath::create` is refused there, before any call.

### Why not a `dyn`

The caller holds a `MethodPath` value, not a trait object, and the transport is a generic parameter as for `Transport`. A consumer that needs one object-safe boundary wraps `RawTransport` itself, as the doc of `Transport` already says for the typed trait. This proposal does not decide that boundary; it removes the reason a consumer had to leave the contract.

## 5. Tests the contract must carry (in memory, no dependency)

Each can fail if the feature is broken:

- `MethodPath::create` accepts `/pkg.Service/Method`; refuses empty text, no leading slash, one segment, three segments, an empty service or method, a space, a newline.
- An in-memory `RawTransport` returns the bytes it was given for each of the four shapes, and the path it saw equals the path given.
- A `RawTransport` that wraps another keeps `type Metadata = T::Metadata` (the hook pattern of `Transport`'s doc), shown by a test.
- A typed `Transport` and a `RawTransport` implemented by the same in-memory type agree: `call::<M>` and `call_raw(M::PATH, encoded)` produce the same bytes on the wire (a differential test; it protects the claim that the typed call is a layer over the raw one).
- A conformance row per behaviour in `docs/3-design/compliance/grpc_spec_conformance.md`: the same rows as the typed call, for the raw one.

## 6. Work in `proto-svc` after acceptance

`TonicTransport` implements `RawTransport` over the same engine: the path it already parses from `M::PATH` is parsed from the `MethodPath` instead, and the codec is the byte codec of the typed path with a pass-through `Payload`. Tests against the real `tonic` server that exist for the typed call are repeated for the raw one (the 24-row status table, deadline, size limit, compression, TLS and mTLS, a reply with no `grpc-status`), and the mutation list is extended with one mutant per behaviour. No new engine and no new dependency.

## 7. Effect on `edge-transport-grpc-egress` (#18)

- `GrpcRequest`, `GrpcResponse`, `GrpcEgressError` and `GrpcStatusCode` become thin or disappear: a raw call is `CallRequest<Vec<u8>>` and `CallResponse<Vec<u8>, _>`, an error is `CallError`.
- Retry, breaker, interceptors, trace context, load balancing and bearer auth are generic wrappers over `RawTransport` (and over `Transport` for typed use), with no `dyn`.
- The metadata of a request is `Metadata`, not `HashMap<String, String>`: a key with an upper-case letter, or ending in `-bin`, is refused by the contract (`MetadataEntry`), and binary entries have their own type. Consumers that passed such keys need to change them.
- Cancellation and the security context stay as ADR-008 recommends (a wrapper race and an interceptor).

## 8. Version and compatibility

`proto-pattern` 0.9.0: one new value, one new error, one new trait; no existing item changes, so every current implementer and consumer compiles unchanged. `proto-svc` 0.3.0 adds the implementation.

## 9. Open questions for the owner of `proto-pattern`

1. Should `MethodPath::create` also accept a path without the leading `/` and add it, for callers that write `pkg.Service/Method`? The old API accepted both; the recommendation is to accept only the canonical form and let the caller normalise.
2. Is one `RawTransport` trait with four methods acceptable, or should the shapes be four traits as the typed side has marker traits? A single trait is recommended: there is no `M` to bind a shape to.
3. Does the contract want a `grpc.health.v1` helper, so that `health_check` of the old API is an ordinary call? Task 12 of #18 can do it in the consumer; it is not needed for this proposal.
