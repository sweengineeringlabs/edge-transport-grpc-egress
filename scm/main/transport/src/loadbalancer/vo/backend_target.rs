//! `BackendTarget` -- one backend a balanced client may send a call to.

/// One backend of a load-balanced client.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BackendTarget {
    /// Base URL of the backend, such as `https://api-1.internal:443`. The method path of a call
    /// is appended to it.
    pub url: String,
    /// Relative share of the calls for [`BalancingStrategy::Weighted`]; ignored by the others.
    ///
    /// [`BalancingStrategy::Weighted`]: crate::BalancingStrategy::Weighted
    pub weight: u32,
}
