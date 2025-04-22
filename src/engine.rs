use tokio::sync::broadcast::{self, Sender};
use tokio::task::JoinSet;
use tokio_stream::StreamExt;
use tracing::{error, info};
use std::time::Instant;

use crate::error::{BotError, Result};
use crate::types::{Collector, Executor, Strategy};
use crate::metrics::METRICS;

/// The main orchestrator that manages the flow of events and actions through the system.
/// 
/// The `Engine` is responsible for:
/// - Managing the lifecycle of collectors, strategies, and executors
/// - Coordinating the flow of events from collectors to strategies
/// - Coordinating the flow of actions from strategies to executors
/// - Handling errors and metrics collection
/// 
/// # Type Parameters
/// 
/// * `E` - The type of events that flow through the system
/// * `A` - The type of actions that flow through the system
/// 
/// # Example
/// 
/// ```rust
/// use botcore::{Engine, Result};
/// use botcore::types::{Collector, CollectorStream, Strategy, Executor};
/// use async_trait::async_trait;
/// use tokio_stream;
/// use tracing::{info, error};
/// 
/// #[derive(Debug, Clone)]
/// struct BlockEvent;
/// 
/// #[derive(Debug, Clone)]
/// struct TradeAction;
/// 
/// struct BlockCollector;
/// 
/// #[async_trait]
/// impl Collector<BlockEvent> for BlockCollector {
///     async fn get_event_stream(&self) -> Result<CollectorStream<'_, BlockEvent>> {
///         // Implementation details...
///         let events = Vec::<BlockEvent>::new();
///         Ok(Box::pin(tokio_stream::iter(events)))
///     }
/// }
/// 
/// struct TradingStrategy;
/// 
/// #[async_trait]
/// impl Strategy<BlockEvent, TradeAction> for TradingStrategy {
///     async fn sync_state(&mut self) -> Result<()> {
///         // Implementation details...
///         Ok(())
///     }
///     
///     async fn process_event(&mut self, _event: BlockEvent) -> Vec<TradeAction> {
///         // Implementation details...
///         vec![]
///     }
/// }
/// 
/// struct TradeExecutor;
/// 
/// #[async_trait]
/// impl Executor<TradeAction> for TradeExecutor {
///     async fn execute(&self, _action: TradeAction) -> Result<()> {
///         // Implementation details...
///         Ok(())
///     }
/// }
/// 
/// async fn run_bot() -> Result<()> {
///     // Create a new engine with custom channel capacities
///     let mut engine = Engine::<BlockEvent, TradeAction>::new()
///         .with_event_channel_capacity(1024)
///         .with_action_channel_capacity(1024);
///     
///     // Add components
///     engine.add_collector(Box::new(BlockCollector));
///     engine.add_strategy(Box::new(TradingStrategy));
///     engine.add_executor(Box::new(TradeExecutor));
///     
///     // Run the engine and handle task results
///     match engine.run().await {
///         Ok(mut set) => {
///             while let Some(res) = set.join_next().await {
///                 match res {
///                     Ok(res) => info!("Task completed: {:?}", res),
///                     Err(e) => info!("Task error: {:?}", e),
///                 }
///             }
///             Ok(())
///         }
///         Err(e) => {
///             error!("Engine run error: {:?}", e);
///             Err(e)
///         }
///     }
/// }
/// ```
pub struct Engine<E, A> {
    /// The set of collectors that the engine will use to collect events.
    collectors: Vec<Box<dyn Collector<E>>>,

    /// The set of strategies that the engine will use to process events.
    strategies: Vec<Box<dyn Strategy<E, A>>>,

    /// The set of executors that the engine will use to execute actions.
    executors: Vec<Box<dyn Executor<A>>>,

    /// The capacity of the event channel.
    event_channel_capacity: usize,

    /// The capacity of the action channel.
    action_channel_capacity: usize,
}

impl<E, A> Engine<E, A> {
    /// Creates a new engine with default channel capacities.
    /// 
    /// The default capacities are:
    /// - Event channel: 512 events
    /// - Action channel: 512 actions
    /// 
    /// Use [`Engine::with_event_channel_capacity`] and [`Engine::with_action_channel_capacity`]
    /// to customize these values.
    #[must_use]
    pub fn new() -> Self {
        Self {
            collectors: vec![],
            strategies: vec![],
            executors: vec![],
            event_channel_capacity: 512,
            action_channel_capacity: 512,
        }
    }

    /// Sets the capacity of the event channel.
    /// 
    /// The event channel is used to buffer events between collectors and
    /// strategies. If the channel becomes full, collectors will be backpressured.
    /// 
    /// # Arguments
    /// 
    /// * `capacity` - The maximum number of events that can be buffered
    #[must_use]
    pub fn with_event_channel_capacity(mut self, capacity: usize) -> Self {
        self.event_channel_capacity = capacity;
        self
    }

    /// Sets the capacity of the action channel.
    /// 
    /// The action channel is used to buffer actions between strategies and
    /// executors. If the channel becomes full, strategies will be backpressured.
    /// 
    /// # Arguments
    /// 
    /// * `capacity` - The maximum number of actions that can be buffered
    #[must_use]
    pub fn with_action_channel_capacity(mut self, capacity: usize) -> Self {
        self.action_channel_capacity = capacity;
        self
    }
}

impl<E, A> Default for Engine<E, A> {
    fn default() -> Self {
        Self::new()
    }
}

impl<E, A> Engine<E, A>
where
    E: Send + Clone + 'static + std::fmt::Debug,
    A: Send + Clone + 'static + std::fmt::Debug,
{
    /// Adds a collector to the engine.
    /// 
    /// # Arguments
    /// 
    /// * `collector` - The collector to add
    pub fn add_collector(&mut self, collector: Box<dyn Collector<E>>) {
        self.collectors.push(collector);
    }

    /// Adds a strategy to the engine.
    /// 
    /// # Arguments
    /// 
    /// * `strategy` - The strategy to add
    pub fn add_strategy(&mut self, strategy: Box<dyn Strategy<E, A>>) {
        self.strategies.push(strategy);
    }

    /// Adds an executor to the engine.
    /// 
    /// # Arguments
    /// 
    /// * `executor` - The executor to add
    pub fn add_executor(&mut self, executor: Box<dyn Executor<A>>) {
        self.executors.push(executor);
    }

    /// Runs the bot engine, starting all components and returning a `JoinSet` for task management.
    ///
    /// This function consumes the `Engine` instance.
    ///
    /// # Errors
    ///
    /// Returns an error if any strategy fails its initial `sync_state` call.
    #[allow(clippy::too_many_lines)]
    pub async fn run(self) -> Result<JoinSet<()>> {
        let (event_sender, _): (Sender<E>, _) = broadcast::channel(self.event_channel_capacity);
        let (action_sender, _): (Sender<A>, _) = broadcast::channel(self.action_channel_capacity);

        let mut set = JoinSet::new();

        // Spawn executors in separate threads.
        for (idx, executor) in self.executors.into_iter().enumerate() {
            let mut action_receiver = action_sender.subscribe();
            let executor_label = format!("executor_{idx}");
            
            set.spawn(async move {
                info!("starting executor... ");
                loop {
                    match action_receiver.recv().await {
                        Ok(action) => {
                            let start = Instant::now();
                            if let Err(e) = executor.execute(action).await {
                                METRICS.record_error(&executor_label, "execution_error");
                                error!(
                                    error = %BotError::executor_error_with_source("Failed to execute action", e),
                                    "executor error"
                                );
                            } else {
                                let duration = start.elapsed().as_secs_f64();

                                METRICS.record_action_execution(&executor_label, duration);
                                METRICS.inc_actions_executed(&executor_label);
                            }
                        }
                        Err(e) => {
                            METRICS.record_error(&executor_label, "channel_error");
                            error!(
                                error = %BotError::channel_error(format!("Failed to receive action: {e}")),
                                "channel error"
                            );
                        }
                    }
                }
            });
        }

        // Spawn strategies in separate threads.
        for (idx, mut strategy) in self.strategies.into_iter().enumerate() {
            let mut event_receiver = event_sender.subscribe();
            let action_sender = action_sender.clone();
            let strategy_label = format!("strategy_{idx}");
            
            strategy.sync_state().await.map_err(|e| {
                METRICS.record_error(&strategy_label, "sync_error");
                BotError::strategy_error_with_source("Failed to sync strategy state", e)
            })?;

            set.spawn(async move {
                info!("starting strategy... ");
                loop {
                    match event_receiver.recv().await {
                        Ok(event) => {
                            let start = Instant::now();
                            let actions = strategy.process_event(event).await;
                            let duration = start.elapsed().as_secs_f64();
                            
                            METRICS.record_event_processing(&strategy_label, duration);

                            for action in actions {
                                if let Err(e) = action_sender.send(action) {
                                    METRICS.record_error(&strategy_label, "channel_error");
                                    error!(
                                        error = %BotError::channel_error(format!("Failed to send action: {e}")),
                                        "channel error"
                                    );
                                }
                            }

                            // Update queue size metrics
                            METRICS.update_action_queue_size(&strategy_label, action_sender.len().try_into().unwrap_or(i64::MAX));
                        }
                        Err(e) => {
                            METRICS.record_error(&strategy_label, "channel_error");
                            error!(
                                error = %BotError::channel_error(format!("Failed to receive event: {e}")),
                                "channel error"
                            );
                        }
                    }
                }
            });
        }

        // Spawn collectors in separate threads.
        for (idx, collector) in self.collectors.into_iter().enumerate() {
            let event_sender = event_sender.clone();
            let collector_label = format!("collector_{idx}");
            
            set.spawn(async move {
                info!("starting collector... ");
                let mut event_stream = match collector.get_event_stream().await {
                    Ok(stream) => stream,
                    Err(e) => {
                        METRICS.record_error(&collector_label, "stream_error");
                        error!(
                            error = %BotError::collector_error_with_source("Failed to get event stream", e),
                            "collector error"
                        );
                        return;
                    }
                };

                while let Some(event) = event_stream.next().await {
                    if let Err(e) = event_sender.send(event) {
                        METRICS.record_error(&collector_label, "channel_error");
                        error!(
                            error = %BotError::channel_error(format!("Failed to send event: {e}")),
                            "channel error"
                        );
                    } else {
                        METRICS.inc_events_processed(&collector_label);
                        
                        // Update queue size metrics
                        METRICS.update_event_queue_size(&collector_label, event_sender.len().try_into().unwrap_or(i64::MAX));
                    }
                }
            });
        }

        Ok(set)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::CollectorStream;
    use async_trait::async_trait;
    use std::sync::Arc;
    use tokio::sync::Mutex;

    #[derive(Debug, Clone)]
    struct TestEvent(u64);

    #[allow(dead_code)]
    #[derive(Debug, Clone)]    

    struct TestAction(String);

    // Mock collector that can be configured to fail
    struct MockCollector {
        should_fail: bool,
    }

    #[async_trait]
    impl Collector<TestEvent> for MockCollector {
        async fn get_event_stream(&self) -> Result<CollectorStream<'_, TestEvent>> {
            if self.should_fail {
                Err(BotError::collector_error("Failed to get event stream"))
            } else {
                let events = vec![TestEvent(1)];
                Ok(Box::pin(tokio_stream::iter(events)))
            }
        }
    }

    // Mock strategy that can be configured to fail
    struct MockStrategy {
        should_fail_sync: bool,
        should_fail_process: bool,
    }

    #[async_trait]
    impl Strategy<TestEvent, TestAction> for MockStrategy {
        async fn sync_state(&mut self) -> Result<()> {
            if self.should_fail_sync {
                Err(BotError::strategy_error("Failed to sync state"))
            } else {
                Ok(())
            }
        }

        async fn process_event(&mut self, event: TestEvent) -> Vec<TestAction> {
            if self.should_fail_process {
                vec![]
            } else {
                vec![TestAction(format!("processed_{}", event.0))]
            }
        }
    }

    // Mock executor that can be configured to fail
    struct MockExecutor {
        should_fail: bool,
        executed_actions: Arc<Mutex<Vec<TestAction>>>,
    }

    #[async_trait]
    impl Executor<TestAction> for MockExecutor {
        async fn execute(&self, action: TestAction) -> Result<()> {
            if self.should_fail {
                Err(BotError::executor_error("Failed to execute action"))
            } else {
                let mut actions = self.executed_actions.lock().await;
                actions.push(action);
                Ok(())
            }
        }
    }

    #[tokio::test]
    async fn test_collector_error() {
        let mut engine = Engine::<TestEvent, TestAction>::new();
        engine.add_collector(Box::new(MockCollector { should_fail: true }));
        engine.add_strategy(Box::new(MockStrategy {
            should_fail_sync: false,
            should_fail_process: false,
        }));
        engine.add_executor(Box::new(MockExecutor {
            should_fail: false,
            executed_actions: Arc::new(Mutex::new(Vec::new())),
        }));

        let join_set = engine.run().await.unwrap();
        
        // The collector task should exit with an error
        assert_eq!(join_set.len(), 3); // All tasks should be spawned
    }

    #[tokio::test]
    async fn test_strategy_sync_error() {
        let mut engine = Engine::<TestEvent, TestAction>::new();
        engine.add_collector(Box::new(MockCollector { should_fail: false }));
        engine.add_strategy(Box::new(MockStrategy {
            should_fail_sync: true,
            should_fail_process: false,
        }));
        engine.add_executor(Box::new(MockExecutor {
            should_fail: false,
            executed_actions: Arc::new(Mutex::new(Vec::new())),
        }));

        let result = engine.run().await;
        assert!(result.is_err());
        if let Err(BotError::StrategyError { message, .. }) = result {
            assert!(message.contains("Failed to sync strategy state"), "Unexpected error message: {message}");
        } else {
            panic!("Expected StrategyError");
        }
    }

    #[tokio::test]
    async fn test_executor_error() {
        let mut engine = Engine::<TestEvent, TestAction>::new();
        engine.add_collector(Box::new(MockCollector { should_fail: false }));
        engine.add_strategy(Box::new(MockStrategy {
            should_fail_sync: false,
            should_fail_process: false,
        }));
        engine.add_executor(Box::new(MockExecutor {
            should_fail: true,
            executed_actions: Arc::new(Mutex::new(Vec::new())),
        }));

        let join_set = engine.run().await.unwrap();
        
        // Let the engine run for a bit to process events
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        
        // The executor should have logged errors but continued running
        assert!(!join_set.is_empty());
    }

    #[tokio::test]
    async fn test_channel_capacity() {
        let mut engine = Engine::<TestEvent, TestAction>::new()
            .with_event_channel_capacity(1)
            .with_action_channel_capacity(1);

        // Add a slow executor to test channel backpressure
        let executed_actions = Arc::new(Mutex::new(Vec::new()));
        engine.add_collector(Box::new(MockCollector { should_fail: false }));
        engine.add_strategy(Box::new(MockStrategy {
            should_fail_sync: false,
            should_fail_process: false,
        }));
        engine.add_executor(Box::new(MockExecutor {
            should_fail: false,
            executed_actions: Arc::clone(&executed_actions),
        }));

        let join_set = engine.run().await.unwrap();
        
        // Let the engine run for a bit
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        
        // Verify that some actions were processed despite the small channel capacity
        let actions = executed_actions.lock().await;
        assert!(!actions.is_empty());
        
        // The engine should still be running
        assert!(!join_set.is_empty());
    }

    #[tokio::test]
    async fn test_empty_engine() {
        let engine = Engine::<TestEvent, TestAction>::new();
        let join_set = engine.run().await.unwrap();
        assert_eq!(join_set.len(), 0);
    }

    #[tokio::test]
    async fn test_multiple_components() {
        let mut engine = Engine::<TestEvent, TestAction>::new();
        
        // Add multiple collectors, strategies, and executors
        for _ in 0..3 {
            engine.add_collector(Box::new(MockCollector { should_fail: false }));
            engine.add_strategy(Box::new(MockStrategy {
                should_fail_sync: false,
                should_fail_process: false,
            }));
            engine.add_executor(Box::new(MockExecutor {
                should_fail: false,
                executed_actions: Arc::new(Mutex::new(Vec::new())),
            }));
        }

        let join_set = engine.run().await.unwrap();
        
        // Let the engine run for a bit
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        
        // Verify that all components are running
        assert_eq!(join_set.len(), 9); // 3 collectors + 3 strategies + 3 executors
    }

    // Add test for strategy process error
    #[tokio::test]
    async fn test_strategy_process_error() {
        let mut engine = Engine::<TestEvent, TestAction>::new();
        engine.add_collector(Box::new(MockCollector { should_fail: false }));
        engine.add_strategy(Box::new(MockStrategy {
            should_fail_sync: false,
            should_fail_process: true,
        }));
        engine.add_executor(Box::new(MockExecutor {
            should_fail: false,
            executed_actions: Arc::new(Mutex::new(Vec::new())),
        }));

        let join_set = engine.run().await.unwrap();
        
        // Let the engine run for a bit
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        
        // The strategy should continue running but not produce any actions
        assert!(!join_set.is_empty());
    }
}
