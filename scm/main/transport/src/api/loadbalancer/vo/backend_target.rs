//! `BackendTarget` -- one backend a balanced client may send a call to.

use serde::{Deserialize, Serialize};

/// One backend of a load-balanced client.
///
/// ```compile_fail,E0451
/// use grpc_egress_transport::BackendTarget;
/// let _ = BackendTarget { url: String::new(), weight: 0 };
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "BackendTargetData")]
pub struct BackendTarget {
    /// Base URL of the backend, such as `https://api-1.internal:443`. The method path of a call
    /// is appended to it.
    url: String,
    /// Relative share of the calls for [`BalancingStrategy::Weighted`]; ignored by the others.
    ///
    /// [`BalancingStrategy::Weighted`]: crate::BalancingStrategy::Weighted
    weight: u32,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct BackendTargetData {
    url: String,
    weight: u32,
}
impl TryFrom<BackendTargetData> for BackendTarget {
    type Error = String;
    fn try_from(v: BackendTargetData) -> Result<Self, String> {
        Self::try_new(v.url, v.weight)
    }
}
impl BackendTarget {
    /// Construct a backend destination with a positive load-balancing weight.
    pub fn try_new(url: impl Into<String>, weight: u32) -> Result<Self, String> {
        let url = url.into();
        if url.trim().is_empty() {
            return Err("backend url must not be empty".into());
        }
        if weight == 0 {
            return Err("backend weight must be >= 1".into());
        }
        Ok(Self { url, weight })
    }
    /// Backend base URI.
    pub fn url(&self) -> &str {
        &self.url
    }
    /// Relative share for weighted balancing.
    pub fn weight(&self) -> u32 {
        self.weight
    }
}
