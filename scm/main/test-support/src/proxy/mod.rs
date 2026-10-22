//! A TCP proxy that a test can freeze, to make a healthy peer stop answering.

mod freezable_proxy;
mod running_proxy;

pub use freezable_proxy::FreezableProxy;
pub use running_proxy::RunningProxy;
