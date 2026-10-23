//! Provider-backed client and transport wrappers.

pub(crate) mod intercepted_transport;
pub(crate) mod pending_call;
pub(crate) mod protosvc;
pub(crate) mod std_metadata_extender;

pub(crate) use intercepted_transport::InterceptedTransport;
pub(crate) use pending_call::PendingCall;
pub(crate) use std_metadata_extender::StdMetadataExtender;
