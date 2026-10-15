//! Tonic-backed load-balancer SPI adapter.
pub(crate) mod lb_grpc_egress;
pub(crate) use lb_grpc_egress::TonicLbGrpcEgress;
