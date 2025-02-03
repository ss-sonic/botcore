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
/// use botcore::types::{Collector, Strategy, Executor};
/// 
/// # struct BlockEvent;
/// # struct TradeAction;
/// # struct BlockCollector;
/// # struct TradingStrategy;
/// # struct TradeExecutor;
/// # impl Collector<BlockEvent> for BlockCollector { }
/// # impl Strategy<BlockEvent, TradeAction> for TradingStrategy { }
/// # impl Executor<TradeAction> for TradeExecutor { }
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
///     // Run the engine
///     let join_set = engine.run().await?;
///     
///     // Wait for all tasks to complete
///     join_set.await;
///     Ok(())
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
    /// Use [`with_event_channel_capacity`] and [`with_action_channel_capacity`]
    /// to customize these values.
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

    /// Starts the engine and returns a set of tasks that can be awaited.
    /// 
    /// This method:
    /// 1. Creates channels for events and actions
    /// 2. Spawns tasks for each collector, strategy, and executor
    /// 3. Returns a [`JoinSet`] containing all spawned tasks
    /// 
    /// The engine will continue running until one of:
    /// - A collector's event stream ends
    /// - A fatal error occurs
    /// - The returned [`JoinSet`] is dropped
    /// 
    /// # Returns
    /// 
    /// A [`JoinSet`] containing all spawned tasks. The caller should await this
    /// set to keep the engine running.
    /// 
    /// # Errors
    /// 
    /// This method will return an error if any strategy fails to sync its initial
    /// state.
    pub async fn run(self) -> Result<JoinSet<()>> {
        let (event_sender, _): (Sender<E>, _) = broadcast::channel(self.event_channel_capacity);
        let (action_sender, _): (Sender<A>, _) = broadcast::channel(self.action_channel_capacity);

        let mut set = JoinSet::new();

        // Spawn executors in separate threads.
        for (idx, executor) in self.executors.into_iter().enumerate() {
            let mut receiver = action_sender.subscribe();
            let executor_label = format!("executor_{}", idx);
            
            set.spawn(async move {
                info!("starting executor... ");
                loop {
                    match receiver.recv().await {
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
                                error = %BotError::channel_error(format!("Failed to receive action: {}", e)),
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
            let strategy_label = format!("strategy_{}", idx);
            
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
                                        error = %BotError::channel_error(format!("Failed to send action: {}", e)),
                                        "channel error"
                                    );
                                }
                            }

                            // Update queue size metrics
                            METRICS.update_action_queue_size(&strategy_label, action_sender.len() as i64);
                        }
                        Err(e) => {
                            METRICS.record_error(&strategy_label, "channel_error");
                            error!(
                                error = %BotError::channel_error(format!("Failed to receive event: {}", e)),
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
            let collector_label = format!("collector_{}", idx);
            
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
                            error = %BotError::channel_error(format!("Failed to send event: {}", e)),
                            "channel error"
                        );
                    } else {
                        METRICS.inc_events_processed(&collector_label);
                        
                        // Update queue size metrics
                        METRICS.update_event_queue_size(&collector_label, event_sender.len() as i64);
                    }
                }
            });
        }

        Ok(set)
    }
}
