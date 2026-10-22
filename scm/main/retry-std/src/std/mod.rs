//! `std` layer: the standard implementation, defaults only. It names the domain crate, `core`, `std` and `alloc`, and no
//! third-party crate; code that needs one is in `spi/`.

pub(crate) mod std_retry_classifier;

pub(crate) use std_retry_classifier::StdRetryClassifier;
