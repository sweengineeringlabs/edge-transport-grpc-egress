//! `jsonwebtoken`-backed JWT signing implementation of the bearer egress interceptor contract.

pub(crate) mod bearer_egress_interceptor;

pub(crate) use bearer_egress_interceptor::BearerEgressInterceptor;
