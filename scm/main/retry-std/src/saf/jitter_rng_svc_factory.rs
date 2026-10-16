//! Composition site for [`JitterRng`] — one file per trait keeps wiring focused.

use edge_transport_retry::DefaultJitterRng;

use grpc_egress_retry::JitterRng;

/// Factory for the default [`JitterRng`].
pub struct JitterRngFactory;

impl JitterRngFactory {
    /// Construct the default [`JitterRng`], seeded from the wall clock.
    pub fn create() -> Box<dyn JitterRng> {
        Box::new(DefaultJitterRng::from_clock())
    }
}
