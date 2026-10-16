//! `codec` theme: typed unary calls over the byte client (Cargo feature `prost`).

pub(crate) mod prost_unary_caller;

pub use prost_unary_caller::ProstUnaryCaller;
