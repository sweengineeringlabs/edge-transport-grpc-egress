//! Integration tests for `api/interceptor/grpc/grpc_egress_interceptor_chain.rs`.

use grpc_egress_transport_std::GrpcRequestFactory;
use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use grpc_egress_transport::{
    AfterCallRequest, GrpcEgressError, GrpcEgressInterceptor, GrpcRequest, GrpcResponse,
    GrpcStatusCode,
};
use grpc_egress_transport_std::GrpcEgressInterceptorChain;

struct Recorder {
    marker: &'static str,
    log: Arc<Mutex<Vec<&'static str>>>,
}

impl GrpcEgressInterceptor for Recorder {
    fn before_call(&self, _: &mut GrpcRequest) -> Result<(), GrpcEgressError> {
        self.log
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .push(self.marker);
        Ok(())
    }
    fn after_call(&self, _: AfterCallRequest<'_>) -> Result<(), GrpcEgressError> {
        self.log
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .push(self.marker);
        Ok(())
    }
}

struct AlwaysFailBefore;

impl GrpcEgressInterceptor for AlwaysFailBefore {
    fn before_call(&self, _: &mut GrpcRequest) -> Result<(), GrpcEgressError> {
        Err(GrpcEgressError::Status(
            GrpcStatusCode::PermissionDenied,
            "denied by interceptor".into(),
        ))
    }
    fn after_call(&self, _: AfterCallRequest<'_>) -> Result<(), GrpcEgressError> {
        Ok(())
    }
}

struct CountAfter(Arc<AtomicUsize>);

impl GrpcEgressInterceptor for CountAfter {
    fn before_call(&self, _: &mut GrpcRequest) -> Result<(), GrpcEgressError> {
        Ok(())
    }
    fn after_call(&self, _: AfterCallRequest<'_>) -> Result<(), GrpcEgressError> {
        self.0.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }
}

fn req() -> GrpcRequest {
    GrpcRequest::new("svc/M", vec![], Duration::from_secs(1))
}

fn resp() -> GrpcResponse {
    GrpcResponse {
        body: vec![],
        metadata: HashMap::new(),
    }
}

#[test]
fn transport_trait_new_chain_is_empty_int_test() {
    let chain = GrpcEgressInterceptorChain::new();
    assert_eq!(chain.len(), 0);
    assert!(chain.is_empty());
}

#[test]
fn transport_trait_push_appends_in_registration_order_int_test(
) -> Result<(), Box<dyn std::error::Error>> {
    let log = Arc::new(Mutex::new(Vec::new()));
    let chain = GrpcEgressInterceptorChain::new()
        .push(Arc::new(Recorder {
            marker: "a",
            log: log.clone(),
        }))
        .push(Arc::new(Recorder {
            marker: "b",
            log: log.clone(),
        }))
        .push(Arc::new(Recorder {
            marker: "c",
            log: log.clone(),
        }));
    let mut r = req();
    chain.run_before(&mut r)?;
    assert_eq!(
        log.lock().map_err(|e| e.to_string())?.clone(),
        vec!["a", "b", "c"]
    );
    Ok(())
}

#[test]
fn transport_trait_run_before_short_circuits_on_first_failure_int_test() {
    let after_count = Arc::new(AtomicUsize::new(0));
    let chain = GrpcEgressInterceptorChain::new()
        .push(Arc::new(AlwaysFailBefore))
        .push(Arc::new(CountAfter(after_count.clone())));
    let mut r = req();
    match chain.run_before(&mut r) {
        Err(GrpcEgressError::Status(code, _)) => {
            assert_eq!(code, GrpcStatusCode::PermissionDenied);
        }
        other => panic!("expected PermissionDenied, got {other:?}"),
    }
    assert_eq!(after_count.load(Ordering::SeqCst), 0);
}

#[test]
fn transport_trait_run_after_invokes_every_interceptor_in_order_int_test(
) -> Result<(), Box<dyn std::error::Error>> {
    let log = Arc::new(Mutex::new(Vec::new()));
    let chain = GrpcEgressInterceptorChain::new()
        .push(Arc::new(Recorder {
            marker: "x",
            log: log.clone(),
        }))
        .push(Arc::new(Recorder {
            marker: "y",
            log: log.clone(),
        }));
    let mut r = resp();
    chain.run_after(&mut r)?;
    assert_eq!(
        log.lock().map_err(|e| e.to_string())?.clone(),
        vec!["x", "y"]
    );
    Ok(())
}

#[test]
fn transport_trait_len_returns_number_of_registered_interceptors_int_test() {
    let chain = GrpcEgressInterceptorChain::new().push(Arc::new(AlwaysFailBefore));
    assert_eq!(chain.len(), 1);
    assert!(!chain.is_empty());
}
