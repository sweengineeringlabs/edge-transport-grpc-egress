//! `impl` block for [`BackoffSchedule`]. The type *declaration* lives
//! in `api/`.

use crate::std::BackoffSchedule;

use std::time::Duration;

impl BackoffSchedule {
    /// Wrap a [`Duration`] as a [`BackoffSchedule`].
    pub fn from_duration(sleep: Duration) -> Self {
        Self { sleep }
    }
}
