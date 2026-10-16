//! The load balancer over `proto-svc` clients: one client per backend.
pub(crate) mod balanced_grpc_egress;

pub(crate) use balanced_grpc_egress::BalancedGrpcEgress;
