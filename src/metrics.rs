// src/metrics.rs

#[cfg(feature = "metrics")]
mod metrics_impl {
    use once_cell::sync::Lazy;
    use prometheus::{
        register_histogram_vec, register_int_counter_vec, register_int_gauge_vec, HistogramVec,
        IntCounterVec, IntGaugeVec,
    };

    /// Metrics collection and reporting for the bot engine.
    ///
    /// This module provides a simple metrics implementation that can be replaced
    /// with a more sophisticated one in production environments.
    pub struct Metrics {
        /// The histogram for recording the duration of action executions.
        pub action_execution_duration: HistogramVec,
        /// The counter for recording the total number of actions executed.
        pub actions_executed_total: IntCounterVec,
        /// The counter for recording error occurrences.
        pub error_count: IntCounterVec,
        /// The histogram for recording the duration of event processing.
        pub event_processing_duration: HistogramVec,
        /// The gauge for recording the current size of the event queue.
        pub event_queue_size: IntGaugeVec,
        /// The counter for recording the total number of events processed.
        pub events_processed_total: IntCounterVec,
        /// The gauge for recording the current size of the action queue.
        pub action_queue_size: IntGaugeVec,
    }

    impl Metrics {
        /// Creates a new metrics instance.
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

        /// Records the duration of an action execution.
        ///
        /// # Arguments
        ///
        /// * `executor_label` - The label identifying the executor
        /// * `duration` - The duration of the action execution in seconds
        pub fn record_action_execution(&self, executor_label: &str, duration: f64) {
            self.action_execution_duration
                .with_label_values(&[executor_label])
                .observe(duration);
        }

        /// Increments the count of executed actions.
        ///
        /// # Arguments
        ///
        /// * `executor_label` - The label identifying the executor
        pub fn inc_actions_executed(&self, executor_label: &str) {
            self.actions_executed_total
                .with_label_values(&[executor_label])
                .inc();
        }

        /// Records an error occurrence.
        ///
        /// # Arguments
        ///
        /// * `component` - The component where the error occurred
        /// * `error_type` - The type of error that occurred
        pub fn record_error(&self, component: &str, error_type: &str) {
            self.error_count
                .with_label_values(&[component, error_type])
                .inc();
        }

        /// Records the duration of event processing.
        ///
        /// # Arguments
        ///
        /// * `strategy_label` - The label identifying the strategy
        /// * `duration` - The duration of event processing in seconds
        pub fn record_event_processing(&self, strategy_label: &str, duration: f64) {
            self.event_processing_duration
                .with_label_values(&[strategy_label])
                .observe(duration);
        }

        /// Updates the current size of the event queue.
        ///
        /// # Arguments
        ///
        /// * `collector_label` - The label identifying the collector
        /// * `size` - The current size of the queue
        pub fn update_event_queue_size(&self, collector_label: &str, size: i64) {
            self.event_queue_size
                .with_label_values(&[collector_label])
                .set(size);
        }

        /// Increments the count of processed events.
        ///
        /// # Arguments
        ///
        /// * `collector_label` - The label identifying the collector
        pub fn inc_events_processed(&self, collector_label: &str) {
            self.events_processed_total
                .with_label_values(&[collector_label])
                .inc();
        }

        /// Updates the current size of the action queue.
        ///
        /// # Arguments
        ///
        /// * `strategy_label` - The label identifying the strategy
        /// * `size` - The current size of the queue
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
    /// A mock metrics implementation for testing.
    ///
    /// This struct provides no-op implementations of all metrics methods
    /// to facilitate testing without requiring a real metrics backend.
    pub struct Metrics;

    impl Metrics {
        /// Creates a new mock metrics instance.
        pub fn new() -> Self {
            Metrics
        }

        /// Records the duration of an action execution.
        ///
        /// # Arguments
        ///
        /// * `executor_label` - The label identifying the executor
        /// * `duration` - The duration of the action execution in seconds
        pub fn record_action_execution(&self, _executor_label: &str, _duration: f64) {}

        /// Increments the count of executed actions.
        ///
        /// # Arguments
        ///
        /// * `executor_label` - The label identifying the executor
        pub fn inc_actions_executed(&self, _executor_label: &str) {}

        /// Records an error occurrence.
        ///
        /// # Arguments
        ///
        /// * `component` - The component where the error occurred
        /// * `error_type` - The type of error that occurred
        pub fn record_error(&self, _component: &str, _error_type: &str) {}

        /// Records the duration of event processing.
        ///
        /// # Arguments
        ///
        /// * `strategy_label` - The label identifying the strategy
        /// * `duration` - The duration of event processing in seconds
        pub fn record_event_processing(&self, _strategy_label: &str, _duration: f64) {}

        /// Updates the current size of the event queue.
        ///
        /// # Arguments
        ///
        /// * `collector_label` - The label identifying the collector
        /// * `size` - The current size of the queue
        pub fn update_event_queue_size(&self, _collector_label: &str, _size: i64) {}

        /// Increments the count of processed events.
        ///
        /// # Arguments
        ///
        /// * `collector_label` - The label identifying the collector
        pub fn inc_events_processed(&self, _collector_label: &str) {}

        /// Updates the current size of the action queue.
        ///
        /// # Arguments
        ///
        /// * `strategy_label` - The label identifying the strategy
        /// * `size` - The current size of the queue
        pub fn update_action_queue_size(&self, _strategy_label: &str, _size: i64) {}
    }

    /// The global metrics instance used throughout the crate.
    ///
    /// This is a mock implementation used for testing that provides
    /// no-op implementations of all metrics methods.
    pub static METRICS: Metrics = Metrics;
}

#[cfg(not(feature = "metrics"))]
pub use metrics_stub::*;
