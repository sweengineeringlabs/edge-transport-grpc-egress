# Architecture: edge-transport-grpc-egress

## Sequence

> A caller makes a call on the client the builder returned: a `proto-pattern` `Transport` (a call over a `Method`) and
> `RawTransport` (a call to a path, with bytes). The interceptor chain runs before the call and again after a response
> with a message; it is empty unless interceptors were attached. The engine (`proto-svc`) owns the wire.

```mermaid
sequenceDiagram
    participant Caller
    participant Client as InterceptedTransport
    participant Endpoint as EndpointTransport
    participant Engine as proto-svc engine
    participant RemoteService

    Caller->>Client: call_raw(MethodPath, CallRequest) or call::<M>(CallRequest)
    Client->>Client: before_call of each interceptor, in order
    Note over Client: for example the trace-context interceptor adds traceparent and tracestate
    Client->>Endpoint: the call, with the headers the interceptors added
    Endpoint->>Endpoint: pick a backend (load balancer) or the one endpoint
    Endpoint->>Engine: the call, as the kind it was
    Engine->>RemoteService: gRPC call over HTTP/2 (TLS)
    RemoteService-->>Engine: response, headers and trailers
    Engine-->>Endpoint: Result of CallResponse or CallError
    Endpoint->>Endpoint: report the outcome to the load balancer
    Endpoint-->>Client: Result
    Client->>Client: after_call of each interceptor, in order
    Client-->>Caller: Result of CallResponse or CallError
```

The types of the client are private to the crate; the builder's `build()` returns them as
`impl Transport + RawTransport`. Retry, a circuit breaker, a cancellation token and a security context wrap that
value without knowing its type ([ADR-009](adr/ADR-009-the-egress-api-on-the-contract.md)).

## Data Flow

> A call is either typed (the caller's `Payload` encodes and decodes the message) or raw (bytes in, bytes out). The
> transport serialises nothing of its own.

```mermaid
flowchart LR
    A["CallRequest\n───────────\nmessage (typed or bytes)\nCallOptions: timeout, Metadata"] --> B["interceptors\nbefore_call"]
    B --> C["endpoint\n(load balancer or one backend)"]
    C --> D["proto-svc engine\nHTTP/2, TLS required by default"]
    D --> E["Remote gRPC service"]
    E --> F["response message\nheaders and trailers"]
    F --> G["interceptors\nafter_call"]
    G -->|Ok| H["CallResponse"]
    G -->|Err| I["CallError\nStatus / Transport / Encode / Decode\nStatusCode on every variant"]
```
