//! `TraceContextSource` — selects the trace context injection mode.

/// Source of the trace context the interceptor will inject.
#[derive(Clone)]
pub enum TraceContextSource {
    /// No automatic injection — only propagates existing upstream value.
    PassThrough,
    /// Always inject a static `traceparent` (and optional `tracestate`).
    Static {
        /// The W3C `traceparent` header value to inject.
        traceparent: String,
        /// The optional W3C `tracestate` header value to inject.
        tracestate: Option<String>,
    },
}
