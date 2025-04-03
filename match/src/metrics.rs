//! Metrics collection module for the Raft match service
//!
//! This module provides functionality for collecting and exposing service metrics
//! using Prometheus.

use lazy_static::lazy_static;
use prometheus::{CounterVec, HistogramOpts, HistogramVec, Opts, Registry};
use std::time::Instant;

lazy_static! {
    /// Global Prometheus registry instance
    pub static ref REGISTRY_INSTANCE: Registry = Registry::new();

    /// Counter for tracking request counts by method
    pub static ref REQ_COUNTER_VEC: CounterVec =
        CounterVec::new(Opts::new("request_counter", "request counter"), &["method"]).unwrap();

    /// Histogram for tracking method execution times
    pub static ref METHOD_HISTOGRAM_VEC: HistogramVec = HistogramVec::new(
        HistogramOpts::new("method_cost", "method cost"),
        &["method"]
    )
    .unwrap();
}

/// Initializes the metrics registry
///
/// Registers all metric collectors with the global registry
pub fn init_registry() {
    let _ = REGISTRY_INSTANCE.register(Box::new(REQ_COUNTER_VEC.clone()));
    let _ = REGISTRY_INSTANCE.register(Box::new(METHOD_HISTOGRAM_VEC.clone()));
}

/// Records metrics for an async operation
///
/// This function:
/// 1. Records the start time
/// 2. Increments the request counter
/// 3. Executes the provided handler
/// 4. Records the execution time
///
/// # Arguments
///
/// * `method_name` - Name of the method being measured
/// * `handler` - Async function to execute and measure
///
/// # Returns
///
/// Returns the result of the handler function
#[allow(unused)]
pub async fn record_metrics<F, Fut, T>(
    method_name: &'static str,
    handler: F,
) -> Result<T, tonic::Status>
where
    F: FnOnce() -> Fut + Send,
    Fut: std::future::Future<Output = Result<T, tonic::Status>> + Send,
{
    let start = Instant::now();
    REQ_COUNTER_VEC.with_label_values(&[method_name]).inc();
    let result = handler().await;

    let elapsed = start.elapsed();
    METHOD_HISTOGRAM_VEC
        .with_label_values(&[method_name])
        .observe(elapsed.as_secs_f64());

    result
}
