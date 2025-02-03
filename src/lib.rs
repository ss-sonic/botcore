#![warn(unused_crate_dependencies)]
#![deny(unused_must_use, rust_2018_idioms)]
#![doc(test(
    no_crate_inject,
    attr(deny(warnings, rust_2018_idioms), allow(dead_code, unused_variables))
))]

//! A flexible and efficient bot engine framework for building event-driven bots.
//!
//! This crate provides a modular architecture for building bots that can:
//! - Collect events from various sources (e.g., blockchain events, API webhooks)
//! - Process events through customizable strategies
//! - Execute actions based on strategy decisions
//!
//! # Architecture
//!
//! The framework is built around three main traits:
//!
//! - [`Collector`](types::Collector): Sources that produce events
//! - [`Strategy`](types::Strategy): Components that process events and decide on actions
//! - [`Executor`](types::Executor): Components that execute actions
//!
//! These components are orchestrated by the [`Engine`](engine::Engine), which manages
//! the flow of events and actions through the system.
//!
//! # Example
//!
//! ```rust,no_run
//! use botcore::{Engine, Result};
//! use botcore::types::{Collector, CollectorStream, Strategy, Executor};
//! use async_trait::async_trait;
//! use tokio_stream;
//!
//! #[derive(Debug, Clone)]
//! struct MyEvent;
//!
//! #[derive(Debug, Clone)]
//! struct MyAction;
//!
//! struct MyCollector;
//!
//! #[async_trait]
//! impl Collector<MyEvent> for MyCollector {
//!     async fn get_event_stream(&self) -> Result<CollectorStream<'_, MyEvent>> {
//!         // For example purposes, create an empty stream
//!         let events = Vec::<MyEvent>::new();
//!         Ok(Box::pin(tokio_stream::iter(events)))
//!     }
//! }
//!
//! struct MyStrategy;
//!
//! #[async_trait]
//! impl Strategy<MyEvent, MyAction> for MyStrategy {
//!     async fn sync_state(&mut self) -> Result<()> {
//!         Ok(())
//!     }
//!
//!     async fn process_event(&mut self, _event: MyEvent) -> Vec<MyAction> {
//!         vec![]
//!     }
//! }
//!
//! struct MyExecutor;
//!
//! #[async_trait]
//! impl Executor<MyAction> for MyExecutor {
//!     async fn execute(&self, _action: MyAction) -> Result<()> {
//!         Ok(())
//!     }
//! }
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

#[cfg(test)]
use criterion as _;
#[cfg(test)]
use futures as _;

/// Core engine implementation that orchestrates event flow
pub mod engine;

/// Error types and handling
pub mod error;

/// Prometheus metrics for monitoring
pub mod metrics;

/// Core traits and types
pub mod types;

pub use engine::Engine;
pub use error::{BotError, Result};
