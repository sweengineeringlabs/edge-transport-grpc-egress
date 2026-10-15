# Architecture: edge-transport-grpc-egress

## Sequence

> A caller uses `GrpcEgress` to call a downstream gRPC service. The interceptor chain runs before the wire call and again after it. The chain is empty unless interceptors were attached.

```mermaid
sequenceDiagram
    participant Caller
    participant GrpcEgress
    participant TonicGrpcEgress
    participant InterceptorChain
    participant RemoteService

    Caller->>GrpcEgress: call_unary(GrpcRequest)
    GrpcEgress->>TonicGrpcEgress: call_unary(request)
    TonicGrpcEgress->>InterceptorChain: run_before(request)
    Note over InterceptorChain: for example TraceContextGrpcEgressInterceptor injects traceparent and tracestate
    InterceptorChain-->>TonicGrpcEgress: request, possibly changed

    TonicGrpcEgress->>RemoteService: gRPC call over HTTP/2 (TLS)
    RemoteService-->>TonicGrpcEgress: response bytes and metadata

    TonicGrpcEgress->>InterceptorChain: run_after(response)
    InterceptorChain-->>TonicGrpcEgress: response, possibly changed
    TonicGrpcEgress-->>GrpcEgress: Result of GrpcResponse or GrpcEgressError
    GrpcEgress-->>Caller: Result of GrpcResponse or GrpcEgressError
```

`TonicGrpcEgress` is a private type of the crate; callers hold the client as `Arc<dyn GrpcEgress>`.

## Data Flow

> The request is bytes: the caller encodes the message and decodes the response (the optional `prost` feature adds a typed layer). Nothing is serialised by the transport.

```mermaid
flowchart LR
    A["GrpcRequest\n───────────\nmethod\nbody: bytes\nmetadata\ndeadline"] --> B["TonicGrpcEgress"]
    B --> C["interceptors\nrun_before"]
    C --> D["gRPC wire\n(HTTP/2, TLS required by default)"]
    D --> E["Remote gRPC service"]
    E --> F["response bytes\nand metadata"]
    F --> G["interceptors\nrun_after"]
    G -->|Ok| H["GrpcResponse"]
    G -->|Err| I["GrpcEgressError\nStatus / ConnectionFailed / Timeout\nInternal / Unavailable / Cancelled"]
```
