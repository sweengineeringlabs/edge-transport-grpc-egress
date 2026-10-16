//! API of the `codec` theme: typed messages over the byte client (Cargo feature `prost`).

pub mod traits;

pub use traits::GrpcEgressProstCodec;
