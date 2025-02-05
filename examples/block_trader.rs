//! A simple example of using the bot engine to execute trades based on block events.
//!
//! This example demonstrates:
//! - Creating custom event and action types
//! - Implementing the core traits (Collector, Strategy, Executor)
//! - Configuring and running the engine
//!
//! To run this example:
//! ```sh
//! cargo run --example block_trader
//! ```

use async_trait::async_trait;
use botcore::engine::Engine;
use botcore::types::{Collector, CollectorStream, Executor, Strategy};
use botcore::Result;
use std::time::Duration;
use tokio::time;
use tracing::{error, info};

/// A simple event containing block information
#[derive(Debug, Clone)]
struct BlockEvent {
    number: u64,
    timestamp: u64,
}

/// A simple action to execute a trade
#[derive(Debug, Clone)]
struct TradeAction {
    block_number: u64,
    amount: u64,
    timestamp: u64,
}

/// A collector that simulates block events by emitting them at regular intervals
struct BlockCollector {
    interval: Duration,
}

impl BlockCollector {
    fn new(interval: Duration) -> Self {
        Self { interval }
    }
}

#[async_trait]
impl Collector<BlockEvent> for BlockCollector {
    async fn get_event_stream(&self) -> Result<CollectorStream<'_, BlockEvent>> {
        let interval = self.interval;
        let stream = tokio_stream::StreamExt::map(
            tokio_stream::wrappers::IntervalStream::new(tokio::time::interval(interval)),
            |_| {
                static mut BLOCK: u64 = 0;
                unsafe {
                    BLOCK += 1;
                    BlockEvent {
                        number: BLOCK,
                        timestamp: std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .unwrap()
                            .as_secs(),
                    }
                }
            },
        );
        Ok(Box::pin(stream))
    }
}

/// A strategy that generates trade actions for every N blocks
struct TradeStrategy {
    block_interval: u64,
    trade_amount: u64,
    last_trade_block: u64,
}

impl TradeStrategy {
    fn new(block_interval: u64, trade_amount: u64) -> Self {
        Self {
            block_interval,
            trade_amount,
            last_trade_block: 0,
        }
    }
}

#[async_trait]
impl Strategy<BlockEvent, TradeAction> for TradeStrategy {
    async fn sync_state(&mut self) -> Result<()> {
        // In a real implementation, we would load the last trade block from storage
        Ok(())
    }

    async fn process_event(&mut self, event: BlockEvent) -> Vec<TradeAction> {
        if event.number >= self.last_trade_block + self.block_interval {
            self.last_trade_block = event.number;
            vec![TradeAction {
                block_number: event.number,
                amount: self.trade_amount,
                timestamp: event.timestamp,
            }]
        } else {
            vec![]
        }
    }
}

/// A simple executor that prints trade actions
struct TradeExecutor;

#[async_trait]
impl Executor<TradeAction> for TradeExecutor {
    async fn execute(&self, action: TradeAction) -> Result<()> {
        println!(
            "Executing trade of {} units at block {} at {}",
            action.amount, action.block_number, action.timestamp
        );
        Ok(())
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    // Create a new engine
    let mut engine = Engine::new()
        .with_event_channel_capacity(1024)
        .with_action_channel_capacity(1024);

    // Add components
    engine.add_collector(Box::new(BlockCollector::new(Duration::from_secs(1))));
    engine.add_strategy(Box::new(TradeStrategy::new(5, 100)));
    engine.add_executor(Box::new(TradeExecutor));

    // Run the engine
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

    // Keep the engine running for a while
    time::sleep(Duration::from_secs(30)).await;

    Ok(())
}
