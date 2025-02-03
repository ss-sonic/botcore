use async_trait::async_trait;
use botcore::{
    error::Result,
    types::{Collector, CollectorStream, Executor, Strategy},
};
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio_stream::StreamExt;

// Test types
#[derive(Debug, Clone, PartialEq)]
struct BlockEvent {
    number: u64,
    hash: String,
}

#[derive(Debug, Clone, PartialEq)]
struct TradeAction {
    block_number: u64,
    amount: u64,
}

// Mock collector that simulates blockchain events
struct BlockCollector {
    blocks: Vec<BlockEvent>,
}

#[async_trait]
impl Collector<BlockEvent> for BlockCollector {
    async fn get_event_stream(&self) -> Result<CollectorStream<'_, BlockEvent>> {
        let events = self.blocks.clone();
        Ok(Box::pin(tokio_stream::iter(events)))
    }
}

// Trading strategy that generates trade actions based on block events
struct TradingStrategy {
    min_block_interval: u64,
    last_trade_block: Arc<Mutex<u64>>,
    trade_amount: u64,
}

#[async_trait]
impl Strategy<BlockEvent, TradeAction> for TradingStrategy {
    async fn sync_state(&mut self) -> Result<()> {
        let mut last_block = self.last_trade_block.lock().await;
        *last_block = 0;
        Ok(())
    }

    async fn process_event(&mut self, event: BlockEvent) -> Vec<TradeAction> {
        let mut last_block = self.last_trade_block.lock().await;
        if event.number >= *last_block + self.min_block_interval {
            *last_block = event.number;
            vec![TradeAction {
                block_number: event.number,
                amount: self.trade_amount,
            }]
        } else {
            vec![]
        }
    }
}

// Mock executor that records trade actions
struct TradeExecutor {
    executed_trades: Arc<Mutex<Vec<TradeAction>>>,
}

#[async_trait]
impl Executor<TradeAction> for TradeExecutor {
    async fn execute(&self, action: TradeAction) -> Result<()> {
        let mut trades = self.executed_trades.lock().await;
        trades.push(action);
        Ok(())
    }
}

#[tokio::test]
async fn test_trading_bot_integration() {
    // Set up test components
    let collector = BlockCollector {
        blocks: vec![
            BlockEvent {
                number: 1,
                hash: "hash1".to_string(),
            },
            BlockEvent {
                number: 5,
                hash: "hash5".to_string(),
            },
            BlockEvent {
                number: 6,
                hash: "hash6".to_string(),
            },
            BlockEvent {
                number: 10,
                hash: "hash10".to_string(),
            },
        ],
    };

    let last_trade_block = Arc::new(Mutex::new(0));
    let mut strategy = TradingStrategy {
        min_block_interval: 4,
        last_trade_block: Arc::clone(&last_trade_block),
        trade_amount: 100,
    };

    let executed_trades = Arc::new(Mutex::new(Vec::new()));
    let executor = TradeExecutor {
        executed_trades: Arc::clone(&executed_trades),
    };

    // Initialize strategy state
    strategy.sync_state().await.unwrap();
    assert_eq!(*last_trade_block.lock().await, 0);

    // Process events through the pipeline
    let mut event_stream = collector.get_event_stream().await.unwrap();
    while let Some(event) = event_stream.next().await {
        let actions = strategy.process_event(event).await;
        for action in actions {
            executor.execute(action).await.unwrap();
        }
    }

    // Verify executed trades
    let trades = executed_trades.lock().await;
    assert_eq!(trades.len(), 2);

    // Should trade at block 5 (>= 0 + 4)
    assert_eq!(
        trades[0],
        TradeAction {
            block_number: 5,
            amount: 100
        }
    );

    // Should trade at block 10 (>= 5 + 4)
    assert_eq!(
        trades[1],
        TradeAction {
            block_number: 10,
            amount: 100
        }
    );
}

#[tokio::test]
async fn test_error_propagation() {
    use botcore::error::BotError;

    // Create a collector that simulates an error
    struct ErrorCollector;

    #[async_trait]
    impl Collector<BlockEvent> for ErrorCollector {
        async fn get_event_stream(&self) -> Result<CollectorStream<'_, BlockEvent>> {
            Err(BotError::collector_error("Failed to connect"))
        }
    }

    let collector = ErrorCollector;
    let result = collector.get_event_stream().await;
    assert!(result.is_err());

    if let Err(error) = result {
        match error {
            BotError::CollectorError { message, source } => {
                assert_eq!(message, "Failed to connect");
                assert!(source.is_none());
            }
            _ => panic!("Expected CollectorError"),
        }
    }
}

#[tokio::test]
async fn test_concurrent_processing() {
    use futures::stream::FuturesUnordered;
    use std::time::Duration;
    use tokio::time::sleep;

    // Collector that simulates slow event production
    struct SlowCollector {
        events: Vec<BlockEvent>,
    }

    #[async_trait]
    impl Collector<BlockEvent> for SlowCollector {
        async fn get_event_stream(&self) -> Result<CollectorStream<'_, BlockEvent>> {
            let events = self.events.clone();
            Ok(Box::pin(tokio_stream::iter(events)))
        }
    }

    // Strategy that simulates slow processing
    #[derive(Debug, Clone, PartialEq)]
    struct SlowStrategy {
        delay_ms: u64,
    }

    #[async_trait]
    impl Strategy<BlockEvent, TradeAction> for SlowStrategy {
        async fn sync_state(&mut self) -> Result<()> {
            Ok(())
        }

        async fn process_event(&mut self, event: BlockEvent) -> Vec<TradeAction> {
            sleep(Duration::from_millis(self.delay_ms)).await;
            vec![TradeAction {
                block_number: event.number,
                amount: 100,
            }]
        }
    }

    // Set up test components
    let collector = SlowCollector {
        events: vec![
            BlockEvent {
                number: 1,
                hash: "hash1".to_string(),
            },
            BlockEvent {
                number: 2,
                hash: "hash2".to_string(),
            },
            BlockEvent {
                number: 3,
                hash: "hash3".to_string(),
            },
        ],
    };

    let strategy = SlowStrategy { delay_ms: 100 };
    let executed_trades = Arc::new(Mutex::new(Vec::new()));
    let executor = TradeExecutor {
        executed_trades: Arc::clone(&executed_trades),
    };

    // Process events concurrently
    let mut event_stream = collector.get_event_stream().await.unwrap();
    let mut futures = FuturesUnordered::new();

    while let Some(event) = event_stream.next().await {
        let strategy = strategy.clone();
        let executor = &executor;

        futures.push(async move {
            let mut strategy = strategy; // Make the cloned strategy mutable
            let actions = strategy.process_event(event).await;
            for action in actions {
                executor.execute(action).await.unwrap();
            }
        });
    }

    // Wait for all processing to complete
    while let Some(_) = futures.next().await {}

    // Verify all trades were executed
    let trades = executed_trades.lock().await;
    assert_eq!(trades.len(), 3);

    // Verify trades were executed in order despite concurrent processing
    let block_numbers: Vec<u64> = trades.iter().map(|t| t.block_number).collect();
    assert_eq!(block_numbers, vec![1, 2, 3]);
}
