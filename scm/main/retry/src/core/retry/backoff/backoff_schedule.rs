//! `impl` block for [`BackoffSchedule`]. The type *declaration* lives
//! in `api/`.

use std::time::Duration;

use crate::api::BackoffSchedule;

impl BackoffSchedule {
    /// Wrap a [`Duration`] as a [`BackoffSchedule`].
    pub fn from_duration(sleep: Duration) -> Self {
        Self { sleep }
    }
}
