//! `HealthStatus` -- the serving status of `grpc.health.v1.HealthCheckResponse`.

/// The serving status a health service reports.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, prost::Enumeration)]
#[repr(i32)]
pub enum HealthStatus {
    /// The service did not say.
    Unknown = 0,
    /// The service is up.
    Serving = 1,
    /// The service is down.
    NotServing = 2,
    /// The name asked about is not a service of this server (only a `Watch` says this).
    ServiceUnknown = 3,
}
