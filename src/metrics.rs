// src/metrics.rs

#[cfg(feature = "metrics")]
mod metrics_impl {
    use once_cell::sync::Lazy;
    use prometheus::{
        register_histogram_vec, register_int_counter_vec, register_int_gauge_vec, HistogramVec,
        IntCounterVec, IntGaugeVec,
    };

    pub struct Metrics {
        pub action_execution_duration: HistogramVec,
        pub actions_executed_total: IntCounterVec,
        pub error_count: IntCounterVec,
        pub event_processing_duration: HistogramVec,
        pub event_queue_size: IntGaugeVec,
        pub events_processed_total: IntCounterVec,
        pub action_queue_size: IntGaugeVec,
    }

    impl Metrics {
        pub fn new() -> Self {
            Self {
                action_execution_duration: register_histogram_vec!(
                    "botcore_action_execution_duration_seconds",
                    "Time taken to execute actions",
                    &["executor"]
                )
                .expect("failed to create histogram"),
                actions_executed_total: register_int_counter_vec!(
                    "botcore_actions_executed_total",
                    "Total number of actions executed",
                    &["executor"]
                )
                .expect("failed to create counter"),
                error_count: register_int_counter_vec!(
                    "botcore_errors_total",
                    "Total number of errors encountered",
                    &["component", "error_type"]
                )
                .expect("failed to create counter"),
                event_processing_duration: register_histogram_vec!(
                    "botcore_event_processing_duration_seconds",
                    "Time taken to process events through strategies",
                    &["strategy"]
                )
                .expect("failed to create histogram"),
                event_queue_size: register_int_gauge_vec!(
                    "botcore_event_queue_size",
                    "Current size of the event queue",
                    &["collector"]
                )
                .expect("failed to create gauge"),
                events_processed_total: register_int_counter_vec!(
                    "botcore_events_processed_total",
                    "Total number of events processed",
                    &["collector"]
                )
                .expect("failed to create counter"),
                action_queue_size: register_int_gauge_vec!(
                    "botcore_action_queue_size",
                    "Current size of the action queue",
                    &["strategy"]
                )
                .expect("failed to create gauge"),
            }
        }

        // Methods for recording metrics

        pub fn record_action_execution(&self, executor_label: &str, duration: f64) {
            self.action_execution_duration
                .with_label_values(&[executor_label])
                .observe(duration);
        }

        pub fn inc_actions_executed(&self, executor_label: &str) {
            self.actions_executed_total
                .with_label_values(&[executor_label])
                .inc();
        }

        pub fn record_error(&self, component: &str, error_type: &str) {
            self.error_count
                .with_label_values(&[component, error_type])
                .inc();
        }

        pub fn record_event_processing(&self, strategy_label: &str, duration: f64) {
            self.event_processing_duration
                .with_label_values(&[strategy_label])
                .observe(duration);
        }

        pub fn update_event_queue_size(&self, collector_label: &str, size: i64) {
            self.event_queue_size
                .with_label_values(&[collector_label])
                .set(size);
        }

        pub fn inc_events_processed(&self, collector_label: &str) {
            self.events_processed_total
                .with_label_values(&[collector_label])
                .inc();
        }

        pub fn update_action_queue_size(&self, strategy_label: &str, size: i64) {
            self.action_queue_size
                .with_label_values(&[strategy_label])
                .set(size);
        }
    }

    pub static METRICS: Lazy<Metrics> = Lazy::new(|| Metrics::new());
}

#[cfg(feature = "metrics")]
pub use metrics_impl::*;

////////////////////////////////////////////////////
// When the metrics feature is disabled, provide a stub
#[cfg(not(feature = "metrics"))]
mod metrics_stub {
    pub struct Metrics;

    impl Metrics {
        pub fn new() -> Self {
            Metrics
        }
        pub fn record_action_execution(&self, _executor_label: &str, _duration: f64) {}
        pub fn inc_actions_executed(&self, _executor_label: &str) {}
        pub fn record_error(&self, _component: &str, _error_type: &str) {}
        pub fn record_event_processing(&self, _strategy_label: &str, _duration: f64) {}
        pub fn update_event_queue_size(&self, _collector_label: &str, _size: i64) {}
        pub fn inc_events_processed(&self, _collector_label: &str) {}
        pub fn update_action_queue_size(&self, _strategy_label: &str, _size: i64) {}
    }

    pub static METRICS: Metrics = Metrics;
}

#[cfg(not(feature = "metrics"))]
pub use metrics_stub::*;
