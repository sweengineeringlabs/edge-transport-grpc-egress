//! `impl` block for [`KeepAliveConfig`]. The type *declaration* lives in `api/`.

use std::time::Duration;

use crate::api::KeepAliveConfig;

impl Default for KeepAliveConfig {
    fn default() -> Self {
        Self {
            interval: Duration::from_secs(10),
            timeout: Duration::from_secs(20),
            permit_without_calls: false,
        }
    }
}
