# grpc-egress-test-support

**Audience:** Maintainers and test authors for the gRPC egress workspace.

## WHAT

Shared test fixtures for egress crates: an in-process gRPC peer, TLS and mTLS certificates, a reserved closed port, gRPC frame helpers, and a freezable proxy. This crate is not published and is not a production dependency.

## WHY

Reusable real-peer fixtures let tests verify wire behavior, cancellation, TLS, metadata, and failure paths without duplicating server setup.

## HOW

Run the fixture tests from the workspace root with `cargo test -p grpc-egress-test-support`. Other workspace tests consume these fixtures as a development dependency.

## API

| Fixture | Purpose |
|---|---|
| `PeerServer`, `RunningPeer`, `PeerBehaviour`, `RecordedRequest` | Start a gRPC peer and inspect requests. |
| `TestPki`, `TlsServerMode` | Create test certificates and TLS peer settings. |
| `FreezableProxy`, `RunningProxy` | Stop proxy responses while retaining a connection. |
| `Fixture`, `ClosedPort`, `ClientFixture`, `CallFixture`, `OutboundCallFixture` | Build common test inputs and reserve ports. |
| `GrpcFrame`, `DecodedFrame` | Encode and inspect gRPC message framing. |

## Documentation

See the [workspace overview](../../docs/README.md) and [conformance record](../../docs/egress-conformance.md).
