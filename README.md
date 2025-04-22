# botcore

A flexible and efficient bot engine framework for building event-driven bots in Rust.

[![Crates.io](https://img.shields.io/crates/v/botcore)](https://crates.io/crates/botcore)
[![Documentation](https://docs.rs/botcore/badge.svg)](https://docs.rs/botcore)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

## Overview

`botcore` is a production-grade, asynchronous engine for building robust event-driven bots in Rust. It offers enterprise-level observability features, a flexible architecture, and type-safe APIs that enable developers to efficiently create, test, and deploy trading bots, arbitrage systems, and other event-reactive applications.

## Features

- 🚀 **High Performance**

  - Built on top of Tokio for maximum concurrency
  - Efficient event processing pipeline
  - Configurable channel capacities for backpressure control

- 🔧 **Modular Architecture**

  - Plug-and-play components
  - Easy to extend and customize
  - Clear separation of concerns

- 📊 **Observability**

  - Built-in Prometheus metrics
  - Performance monitoring
  - Error tracking and reporting

- 🛠️ **Developer Experience**

  - Type-safe API design
  - Comprehensive documentation
  - Example implementations
  - Clear error messages

- 🔄 **Event Processing**
  - Flexible event collection
  - Customizable processing strategies
  - Reliable action execution
  - State management utilities

## Installation

Add `botcore` to your project using cargo:

```sh
cargo add botcore --features metrics
```

## Architecture

The framework is built around three core components:

### 1. Collectors

Event sources that feed data into your bot:

- Blockchain event listeners
- WebSocket API clients
- Database change streams
- Message queues
- File system watchers

### 2. Strategies

Processing logic that determines bot behavior:

- State management
- Decision making
- Event filtering
- Action generation
- Error handling

### 3. Executors

Action handlers that effect changes:

- Transaction submission
- API calls
- Database operations
- Notifications
- External system integration

## Quick Start

```rust
use botcore::engine::Engine;
use botcore::types::{Collector, Strategy, Executor};
use botcore::Result;
use tracing::{error, info};

// 1. Define your types
#[derive(Debug, Clone)]
struct MyEvent {
    data: String,
}

#[derive(Debug, Clone)]
struct MyAction {
    command: String,
}

// 2. Implement a collector
#[async_trait]
impl Collector<MyEvent> for MyCollector {
    async fn get_event_stream(&self) -> Result<CollectorStream<'_, MyEvent>> {
        // Return your event stream
    }
}

// 3. Implement a strategy
#[async_trait]
impl Strategy<MyEvent, MyAction> for MyStrategy {
    async fn process_event(&mut self, event: MyEvent) -> Vec<MyAction> {
        // Process events and return actions
    }
}

// 4. Implement an executor
#[async_trait]
impl Executor<MyAction> for MyExecutor {
    async fn execute(&self, action: MyAction) -> Result<()> {
        // Execute actions
        Ok(())
    }
}

// 5. Run the engine
#[tokio::main]
async fn main() -> Result<()> {
    let mut engine = Engine::new()
        .with_event_channel_capacity(1024)
        .with_action_channel_capacity(1024);

    engine.add_collector(Box::new(MyCollector));
    engine.add_strategy(Box::new(MyStrategy));
    engine.add_executor(Box::new(MyExecutor));

    match engine.run().await {
        Ok(mut set) => {
            while let Some(res) = set.join_next().await {
                match res {
                    Ok(res) => {
                        info!("res: {:?}", res);
                    }
                    Err(e) => {
                        info!("error: {:?}", e);
                    }
                }
            }
        }
        Err(e) => {
            error!("Engine run error: {:?}", e);
        }
    }
    Ok(())
}
```

## Examples

The `examples/` directory contains working implementations:

- `block_trader.rs`: A bot that executes trades based on blockchain events
  ```sh
  cargo run --example block_trader
  ```

## Monitoring

`botcore` automatically collects the following Prometheus metrics:

| Metric                     | Type      | Description                        |
| -------------------------- | --------- | ---------------------------------- |
| `event_processing_latency` | Histogram | Time taken to process events       |
| `action_execution_latency` | Histogram | Time taken to execute actions      |
| `event_queue_size`         | Gauge     | Current number of pending events   |
| `action_queue_size`        | Gauge     | Current number of pending actions  |
| `error_count`              | Counter   | Total number of errors encountered |
| `events_processed_total`   | Counter   | Total number of events processed   |
| `actions_executed_total`   | Counter   | Total number of actions executed   |

## Configuration

The engine can be configured through builder methods:

```rust
let engine = Engine::new()
    .with_event_channel_capacity(1024)
    .with_action_channel_capacity(1024)
```

## Error Handling

`botcore` provides a comprehensive error type system:

- Clear error messages
- Error categorization
- Error context preservation
- Recovery strategies

## Best Practices

1. **Event Design**

   - Keep events small and focused
   - Include necessary context
   - Use appropriate serialization

2. **Strategy Implementation**

   - Handle state carefully
   - Implement proper error handling
   - Keep processing logic modular

3. **Execution**
   - Implement retries for transient failures
   - Handle rate limiting
   - Log important state changes

## Contributing

We welcome contributions! Please see our [Contributing Guide](CONTRIBUTING.md) for details on:

- Code style
- Pull request process
- Development setup
- Testing requirements

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## Support

- [Documentation](https://docs.rs/botcore)
- [Issue Tracker](https://github.com/yourusername/botcore/issues)
- [Discussions](https://github.com/yourusername/botcore/discussions)
