#![warn(unused_crate_dependencies)]
#![deny(unused_must_use, rust_2018_idioms)]
#![deny(missing_docs)]
#![deny(rustdoc::missing_crate_level_docs)]
#![doc(test(
    no_crate_inject,
    attr(deny(warnings, rust_2018_idioms), allow(dead_code, unused_variables))
))]

//! # botcore
//!
//! A flexible and efficient bot engine framework for building event-driven bots in Rust.
//!
//! ## Overview
//!
//! `botcore` is a high-performance, type-safe framework for building event-driven bots in Rust.
//! It provides a modular architecture that makes it easy to create, test, and maintain bot
//! applications that can react to various events in real-time.
//!
//! ## Features
//!
//! - 🚀 **High Performance**
//!   - Built on top of Tokio for maximum concurrency
//!   - Efficient event processing pipeline
//!   - Configurable channel capacities for backpressure control
//!
//! - 🔧 **Modular Architecture**
//!   - Plug-and-play components
//!   - Easy to extend and customize
//!   - Clear separation of concerns
//!
//! - 📊 **Observability**
//!   - Built-in Prometheus metrics
//!   - Performance monitoring
//!   - Error tracking and reporting
//!
//! ## Architecture
//!
//! The framework is built around three main traits:
//!
//! - [`Collector`](types::Collector): Sources that produce events
//! - [`Strategy`](types::Strategy): Components that process events and decide on actions
//! - [`Executor`](types::Executor): Components that execute actions
//!
//! These components are orchestrated by the [`Engine`], which manages
//! the flow of events and actions through the system.
//!
//! ## Quick Start
//!
//! ```rust,no_run
//! use botcore::{Engine, Result};
//! use botcore::types::{Collector, CollectorStream, Strategy, Executor};
//! use async_trait::async_trait;
//! use tokio_stream;
//!
//! # #[derive(Debug, Clone)]
//! # struct MyEvent;
//! # #[derive(Debug, Clone)]
//! # struct MyAction;
//! # struct MyCollector;
//! # #[async_trait]
//! # impl Collector<MyEvent> for MyCollector {
//! #     async fn get_event_stream(&self) -> Result<CollectorStream<'_, MyEvent>> {
//! #         let events = Vec::<MyEvent>::new();
//! #         Ok(Box::pin(tokio_stream::iter(events)))
//! #     }
//! # }
//! # struct MyStrategy;
//! # #[async_trait]
//! # impl Strategy<MyEvent, MyAction> for MyStrategy {
//! #     async fn sync_state(&mut self) -> Result<()> {
//! #         Ok(())
//! #     }
//! #     async fn process_event(&mut self, _event: MyEvent) -> Vec<MyAction> {
//! #         vec![]
//! #     }
//! # }
//! # struct MyExecutor;
//! # #[async_trait]
//! # impl Executor<MyAction> for MyExecutor {
//! #     async fn execute(&self, _action: MyAction) -> Result<()> {
//! #         Ok(())
//! #     }
//! # }
//!
//! #[tokio::main]
//! async fn main() -> Result<()> {
//!     // Create a new engine with custom channel capacities
//!     let mut engine = Engine::<MyEvent, MyAction>::new()
//!         .with_event_channel_capacity(1024)
//!         .with_action_channel_capacity(1024);
//!     
//!     // Add components
//!     engine.add_collector(Box::new(MyCollector));
//!     engine.add_strategy(Box::new(MyStrategy));
//!     engine.add_executor(Box::new(MyExecutor));
//!     
//!     // Run the engine
//!     let mut join_set = engine.run().await?;
//!     
//!     // Wait for all tasks to complete
//!     while join_set.join_next().await.is_some() {}
//!     Ok(())
//! }
//! ```
//!
//! ## Modules
//!
//! - [`engine`]: Core engine implementation that orchestrates event flow
//! - [`types`]: Core traits and types for building bots
//! - [`error`]: Error types and handling
//! - [`metrics`]: Prometheus metrics for monitoring
//!
//! ## Error Handling
//!
//! The crate uses a custom [`Result`] type that wraps [`BotError`]
//! for comprehensive error handling. All errors are categorized and include context
//! for easier debugging.
//!
//! ## Metrics
//!
//! The engine automatically collects Prometheus metrics for:
//!
//! - Event processing latency
//! - Action execution latency
//! - Queue sizes
//! - Error counts
//! - Total events processed
//! - Total actions executed
//!
//! ## Best Practices
//!
//! 1. **Event Design**
//!    - Keep events small and focused
//!    - Include necessary context
//!    - Use appropriate serialization
//!
//! 2. **Strategy Implementation**
//!    - Handle state carefully
//!    - Implement proper error handling
//!    - Keep processing logic modular
//!
//! 3. **Execution**
//!    - Implement retries for transient failures
//!    - Handle rate limiting
//!    - Log important state changes

#[cfg(test)]
use criterion as _;
#[cfg(test)]
use futures as _;

/// Core engine implementation that orchestrates event flow.
///
/// This module provides the [`Engine`] type which is responsible for:
/// - Managing the lifecycle of collectors, strategies, and executors
/// - Coordinating the flow of events from collectors to strategies
/// - Coordinating the flow of actions from strategies to executors
/// - Handling errors and metrics collection
pub mod engine;

/// Error types and handling.
///
/// This module provides:
/// - [`BotError`]: The main error type
/// - [`Result`]: A type alias for `Result<T, BotError>`
/// - Various error creation and handling utilities
pub mod error;

/// Prometheus metrics for monitoring.
///
/// This module provides metrics collection for:
/// - Event processing latency
/// - Action execution latency
/// - Queue sizes
/// - Error counts
/// - Total events processed
/// - Total actions executed
pub mod metrics;

/// Core traits and types.
///
/// This module provides the core traits:
/// - [`Collector`](types::Collector): Sources that produce events
/// - [`Strategy`](types::Strategy): Components that process events
/// - [`Executor`](types::Executor): Components that execute actions
pub mod types;

pub use engine::Engine;
pub use error::{BotError, Result};
