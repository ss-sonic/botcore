use hyper::{
    service::{make_service_fn, service_fn},
    Body, Request, Response, Server,
};
use lazy_static::lazy_static;
use prometheus::{
    register_histogram_vec, register_int_counter_vec, register_int_gauge_vec, Encoder,
    HistogramVec, IntCounterVec, IntGaugeVec, TextEncoder,
};
use std::net::SocketAddr;
use tracing::info;

lazy_static! {
    // Event processing metrics
    pub static ref EVENT_PROCESSING_DURATION: HistogramVec = register_histogram_vec!(
        "botcore_event_processing_duration_seconds",
        "Time taken to process events through strategies",
        &["strategy"]
    )
    .unwrap();

    pub static ref EVENTS_PROCESSED_TOTAL: IntCounterVec = register_int_counter_vec!(
        "botcore_events_processed_total",
        "Total number of events processed",
        &["collector"]
    )
    .unwrap();

    // Action execution metrics
    pub static ref ACTION_EXECUTION_DURATION: HistogramVec = register_histogram_vec!(
        "botcore_action_execution_duration_seconds",
        "Time taken to execute actions",
        &["executor"]
    )
    .unwrap();

    pub static ref ACTIONS_EXECUTED_TOTAL: IntCounterVec = register_int_counter_vec!(
        "botcore_actions_executed_total",
        "Total number of actions executed",
        &["executor"]
    )
    .unwrap();

    // Queue metrics
    pub static ref EVENT_QUEUE_SIZE: IntGaugeVec = register_int_gauge_vec!(
        "botcore_event_queue_size",
        "Current size of the event queue",
        &["collector"]
    )
    .unwrap();

    pub static ref ACTION_QUEUE_SIZE: IntGaugeVec = register_int_gauge_vec!(
        "botcore_action_queue_size",
        "Current size of the action queue",
        &["strategy"]
    )
    .unwrap();

    // Error metrics
    pub static ref ERROR_COUNT: IntCounterVec = register_int_counter_vec!(
        "botcore_errors_total",
        "Total number of errors encountered",
        &["component", "error_type"]
    )
    .unwrap();
}

/// Returns the current metrics in Prometheus text format
pub fn get_metrics() -> String {
    let encoder = TextEncoder::new();
    let metric_families = prometheus::gather();
    let mut buffer = Vec::new();
    encoder.encode(&metric_families, &mut buffer).unwrap();
    String::from_utf8(buffer).unwrap()
}

/// A server that exposes Prometheus metrics over HTTP
pub struct MetricsServer {
    addr: SocketAddr,
}

impl MetricsServer {
    /// Create a new metrics server that will listen on the given address
    pub fn new(addr: SocketAddr) -> Self {
        Self { addr }
    }

    /// Start the metrics server. This will block until the server is shut down.
    pub async fn run(self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let make_svc = make_service_fn(|_conn| async {
            Ok::<_, hyper::Error>(service_fn(|_req: Request<Body>| async {
                let metrics = get_metrics();
                Ok::<_, hyper::Error>(Response::new(Body::from(metrics)))
            }))
        });

        let server = Server::bind(&self.addr).serve(make_svc);
        info!("Metrics server listening on http://{}/metrics", self.addr);

        server.await?;
        Ok(())
    }
}

/// A recorder handle that can be used to record metrics manually if needed
#[derive(Clone, Debug)]
pub struct MetricsRecorder {
    strategy_label: String,
    executor_label: String,
    collector_label: String,
}

impl MetricsRecorder {
    /// Create a new metrics recorder with the given component labels
    pub fn new(
        strategy_label: impl Into<String>,
        executor_label: impl Into<String>,
        collector_label: impl Into<String>,
    ) -> Self {
        Self {
            strategy_label: strategy_label.into(),
            executor_label: executor_label.into(),
            collector_label: collector_label.into(),
        }
    }

    /// Record event processing duration
    pub fn record_event_processing(&self, duration_secs: f64) {
        EVENT_PROCESSING_DURATION
            .with_label_values(&[&self.strategy_label])
            .observe(duration_secs);
    }

    /// Record action execution duration
    pub fn record_action_execution(&self, duration_secs: f64) {
        ACTION_EXECUTION_DURATION
            .with_label_values(&[&self.executor_label])
            .observe(duration_secs);
    }

    /// Increment events processed counter
    pub fn inc_events_processed(&self) {
        EVENTS_PROCESSED_TOTAL
            .with_label_values(&[&self.collector_label])
            .inc();
    }

    /// Increment actions executed counter
    pub fn inc_actions_executed(&self) {
        ACTIONS_EXECUTED_TOTAL
            .with_label_values(&[&self.executor_label])
            .inc();
    }

    /// Update event queue size
    pub fn set_event_queue_size(&self, size: i64) {
        EVENT_QUEUE_SIZE
            .with_label_values(&[&self.collector_label])
            .set(size);
    }

    /// Update action queue size
    pub fn set_action_queue_size(&self, size: i64) {
        ACTION_QUEUE_SIZE
            .with_label_values(&[&self.strategy_label])
            .set(size);
    }

    /// Record an error
    pub fn record_error(&self, component: &str, error_type: &str) {
        ERROR_COUNT
            .with_label_values(&[component, error_type])
            .inc();
    }
}
