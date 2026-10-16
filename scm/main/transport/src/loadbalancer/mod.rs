//! API of the `loadbalancer` theme: spreading calls over several backends.

pub mod vo;

pub use vo::{BackendTarget, BalancingStrategy, LoadbalancerSettings};
