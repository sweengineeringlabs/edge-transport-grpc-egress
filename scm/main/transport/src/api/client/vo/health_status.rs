//! Serving status from `grpc.health.v1.HealthCheckResponse`.

/// The serving status a health service reports.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[repr(i32)]
pub enum HealthStatus {
    /// The service did not say, or returned an unrecognized status.
    #[default]
    Unknown = 0,
    /// The service is up.
    Serving = 1,
    /// The service is down.
    NotServing = 2,
    /// The service name is not registered (reported by `Watch`).
    ServiceUnknown = 3,
}

impl From<i32> for HealthStatus {
    fn from(value: i32) -> Self {
        match value {
            1 => Self::Serving,
            2 => Self::NotServing,
            3 => Self::ServiceUnknown,
            _ => Self::Unknown,
        }
    }
}
