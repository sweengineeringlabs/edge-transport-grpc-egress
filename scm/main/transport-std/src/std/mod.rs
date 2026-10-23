//! `std` layer: the standard implementation, defaults only. It names the domain crate, `core`, `std` and `alloc`,
//! and no third-party crate; code that needs one is in `spi/`.

pub(crate) mod channel;
pub(crate) mod interceptor;

pub(crate) use channel::{StdGrpcChannelConfigBuilder, StdTlsSettingsResolver};
pub(crate) use interceptor::{StdGrpcEgressInterceptorChain, TraceContextGrpcEgressInterceptor};
