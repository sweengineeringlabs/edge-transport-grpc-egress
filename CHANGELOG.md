# Changelog

## v0.14.0

- **Breaking:** `GrpcEgressInterceptor::on_event` is now required. Implementors must observe lifecycle events or add an explicit no-op method.
- `GrpcEgressInterceptorChain::run_event` is now required so event forwarding or discarding is explicit.
- `before_call` remains the request mutation/rejection hook; `after_call` remains the successful-response inspection/rejection hook; `on_event` remains observation-only across unary and streaming lifecycle events.
- Workspace crate versions advance to `0.11.0` for this source-breaking contract.
