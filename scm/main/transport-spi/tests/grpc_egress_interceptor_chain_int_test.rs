//! Integration tests for `GrpcEgressInterceptorChain`: the order interceptors run in, what happens when one fails,
//! and that one interceptor sees what the one before it did to the call.

use grpc_egress_transport_api::GrpcEgressInterceptorChain as _;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use grpc_egress_test_support::OutboundCallFixture;
use grpc_egress_transport_api::{GrpcEgressInterceptor, InboundReply, OutboundCall};
use proto_pattern::{CallError, Metadata, StatusCode};

/// Records `before:<marker>` and `after:<marker>` in a shared log, in the order the chain calls it.
struct Recorder {
    marker: &'static str,
    log: Arc<Mutex<Vec<String>>>,
}

impl Recorder {
    fn note(&self, phase: &str) {
        self.log
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .push(format!("{phase}:{}", self.marker));
    }
}

impl GrpcEgressInterceptor for Recorder {
    fn before_call(&self, _: &mut dyn OutboundCall) -> Result<(), CallError> {
        self.note("before");
        Ok(())
    }

    fn after_call(&self, _: &InboundReply<'_>) -> Result<(), CallError> {
        self.note("after");
        Ok(())
    }

    fn on_event(
        &self,
        _event: &grpc_egress_transport_api::proto_pattern::CallEvent<
            '_,
            grpc_egress_transport_api::proto_pattern::Metadata,
        >,
    ) {
    }
}

/// Fails in `before_call` with `PermissionDenied`.
struct DenyBefore;

impl GrpcEgressInterceptor for DenyBefore {
    fn before_call(&self, call: &mut dyn OutboundCall) -> Result<(), CallError> {
        Err(CallError::Status {
            path: call.path().into(),
            code: StatusCode::PermissionDenied,
            message: "denied by interceptor".to_string(),
            details: Vec::new(),
            trailers: Metadata::default(),
        })
    }

    fn after_call(&self, _: &InboundReply<'_>) -> Result<(), CallError> {
        Ok(())
    }

    fn on_event(
        &self,
        _event: &grpc_egress_transport_api::proto_pattern::CallEvent<
            '_,
            grpc_egress_transport_api::proto_pattern::Metadata,
        >,
    ) {
    }
}

/// Fails in `after_call` with `DataLoss`.
struct FailAfter;

impl GrpcEgressInterceptor for FailAfter {
    fn before_call(&self, _: &mut dyn OutboundCall) -> Result<(), CallError> {
        Ok(())
    }

    fn after_call(&self, reply: &InboundReply<'_>) -> Result<(), CallError> {
        Err(CallError::Status {
            path: reply.path().into(),
            code: StatusCode::DataLoss,
            message: "refused by interceptor".to_string(),
            details: Vec::new(),
            trailers: Metadata::default(),
        })
    }

    fn on_event(
        &self,
        _event: &grpc_egress_transport_api::proto_pattern::CallEvent<
            '_,
            grpc_egress_transport_api::proto_pattern::Metadata,
        >,
    ) {
    }
}

/// Counts the calls of each hook.
#[derive(Default)]
struct Counter {
    before: AtomicUsize,
    after: AtomicUsize,
}

/// A handle on a [`Counter`] that is an interceptor, so the test keeps its own handle to read.
struct CountingInterceptor(Arc<Counter>);

impl GrpcEgressInterceptor for CountingInterceptor {
    fn before_call(&self, _: &mut dyn OutboundCall) -> Result<(), CallError> {
        self.0.before.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }

    fn after_call(&self, _: &InboundReply<'_>) -> Result<(), CallError> {
        self.0.after.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }

    fn on_event(
        &self,
        _event: &grpc_egress_transport_api::proto_pattern::CallEvent<
            '_,
            grpc_egress_transport_api::proto_pattern::Metadata,
        >,
    ) {
    }
}

/// Adds the header `key: value` in `before_call`.
struct AddHeader {
    key: &'static str,
    value: &'static str,
}

impl GrpcEgressInterceptor for AddHeader {
    fn before_call(&self, call: &mut dyn OutboundCall) -> Result<(), CallError> {
        call.add_header(self.key, self.value)
    }

    fn after_call(&self, _: &InboundReply<'_>) -> Result<(), CallError> {
        Ok(())
    }

    fn on_event(
        &self,
        _event: &grpc_egress_transport_api::proto_pattern::CallEvent<
            '_,
            grpc_egress_transport_api::proto_pattern::Metadata,
        >,
    ) {
    }
}

/// Adds `x-second: seen-first` only when `x-first` is already on the call.
struct AddWhenFirstIsPresent;

impl GrpcEgressInterceptor for AddWhenFirstIsPresent {
    fn before_call(&self, call: &mut dyn OutboundCall) -> Result<(), CallError> {
        if call.has_header("x-first") {
            call.add_header("x-second", "seen-first")?;
        }
        Ok(())
    }

    fn after_call(&self, _: &InboundReply<'_>) -> Result<(), CallError> {
        Ok(())
    }

    fn on_event(
        &self,
        _event: &grpc_egress_transport_api::proto_pattern::CallEvent<
            '_,
            grpc_egress_transport_api::proto_pattern::Metadata,
        >,
    ) {
    }
}

fn call() -> OutboundCallFixture {
    OutboundCallFixture::create("/svc/M")
        .with_timeout(None)
        .with_metadata(Metadata::default())
}

fn reply() -> InboundReply<'static> {
    InboundReply::new("/svc/M", Metadata::default(), Metadata::default())
}

fn recorder(marker: &'static str, log: &Arc<Mutex<Vec<String>>>) -> Arc<Recorder> {
    Arc::new(Recorder {
        marker,
        log: Arc::clone(log),
    })
}

fn read_log(log: &Arc<Mutex<Vec<String>>>) -> Vec<String> {
    log.lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .clone()
}

#[test]
fn test_new_chain_has_no_interceptor() {
    let chain = new_chain();
    assert_eq!(chain.len(), 0);
    assert!(chain.is_empty());
}

#[test]
fn test_len_chain_with_one_interceptor_counts_it() {
    let chain = new_chain().push(Arc::new(DenyBefore));
    assert_eq!(chain.len(), 1);
    assert!(!chain.is_empty());
}

#[test]
fn test_run_before_runs_interceptors_in_registration_order() -> Result<(), CallError> {
    let log = Arc::new(Mutex::new(Vec::new()));
    let chain = new_chain()
        .push(recorder("a", &log))
        .push(recorder("b", &log))
        .push(recorder("c", &log));

    chain.run_before(&mut call())?;

    assert_eq!(read_log(&log), ["before:a", "before:b", "before:c"]);
    Ok(())
}

#[test]
fn test_run_after_runs_interceptors_in_registration_order_not_reversed() -> Result<(), CallError> {
    let log = Arc::new(Mutex::new(Vec::new()));
    let chain = new_chain()
        .push(recorder("x", &log))
        .push(recorder("y", &log));

    chain.run_after(&reply())?;

    assert_eq!(read_log(&log), ["after:x", "after:y"]);
    Ok(())
}

#[test]
fn test_run_before_first_failure_stops_the_chain_and_returns_its_error() {
    let later = Arc::new(Counter::default());
    let chain = new_chain()
        .push(Arc::new(DenyBefore))
        .push(Arc::new(CountingInterceptor(Arc::clone(&later))));

    let result = chain.run_before(&mut call());

    assert!(
        matches!(
            &result,
            Err(CallError::Status { code: StatusCode::PermissionDenied, message, .. })
                if message == "denied by interceptor"
        ),
        "got {result:?}"
    );
    assert_eq!(
        later.before.load(Ordering::SeqCst),
        0,
        "an interceptor after the failing one must not run"
    );
}

#[test]
fn test_run_after_first_failure_stops_the_chain_and_returns_its_error() {
    let later = Arc::new(Counter::default());
    let chain = new_chain()
        .push(Arc::new(FailAfter))
        .push(Arc::new(CountingInterceptor(Arc::clone(&later))));

    let result = chain.run_after(&reply());

    assert!(
        matches!(
            &result,
            Err(CallError::Status {
                code: StatusCode::DataLoss,
                ..
            })
        ),
        "got {result:?}"
    );
    assert_eq!(later.after.load(Ordering::SeqCst), 0);
}

#[test]
fn test_run_before_later_interceptor_sees_the_header_an_earlier_one_added() -> Result<(), CallError>
{
    let chain = new_chain()
        .push(Arc::new(AddHeader {
            key: "x-first",
            value: "1",
        }))
        .push(Arc::new(AddWhenFirstIsPresent));
    let mut call = call();

    chain.run_before(&mut call)?;

    assert_eq!(call.header_values("x-second"), ["seen-first"]);
    Ok(())
}

#[test]
fn test_run_before_interceptor_that_runs_first_does_not_see_a_later_ones_header(
) -> Result<(), CallError> {
    let chain = new_chain()
        .push(Arc::new(AddWhenFirstIsPresent))
        .push(Arc::new(AddHeader {
            key: "x-first",
            value: "1",
        }));
    let mut call = call();

    chain.run_before(&mut call)?;

    assert!(
        !call.has_header("x-second"),
        "the order of registration is the order of effect"
    );
    Ok(())
}

fn new_chain() -> impl grpc_egress_transport_api::GrpcEgressInterceptorChain {
    use grpc_egress_transport_api::GrpcEgressInterceptorChainFactory;
    grpc_egress_transport_std::StdGrpcEgressInterceptorChainFactory
        .build(grpc_egress_transport_api::BuildGrpcEgressInterceptorChainRequest)
}
