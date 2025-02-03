use tokio::sync::broadcast::{self, Sender};
use tokio::task::JoinSet;
use tokio_stream::StreamExt;
use tracing::{error, info};
use std::time::Instant;

use crate::error::{BotError, Result};
use crate::types::{Collector, Executor, Strategy};
use crate::metrics::{
    ACTION_EXECUTION_DURATION, ACTION_QUEUE_SIZE, ACTIONS_EXECUTED_TOTAL,
    ERROR_COUNT, EVENT_PROCESSING_DURATION, EVENT_QUEUE_SIZE, EVENTS_PROCESSED_TOTAL,
};

/// The main engine of Artemis. This struct is responsible for orchestrating the
/// data flow between collectors, strategies, and executors.
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
    pub fn new() -> Self {
        Self {
            collectors: vec![],
            strategies: vec![],
            executors: vec![],
            event_channel_capacity: 512,
            action_channel_capacity: 512,
        }
    }

    pub fn with_event_channel_capacity(mut self, capacity: usize) -> Self {
        self.event_channel_capacity = capacity;
        self
    }

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
    /// Adds a collector to be used by the engine.
    pub fn add_collector(&mut self, collector: Box<dyn Collector<E>>) {
        self.collectors.push(collector);
    }

    /// Adds a strategy to be used by the engine.
    pub fn add_strategy(&mut self, strategy: Box<dyn Strategy<E, A>>) {
        self.strategies.push(strategy);
    }

    /// Adds an executor to be used by the engine.
    pub fn add_executor(&mut self, executor: Box<dyn Executor<A>>) {
        self.executors.push(executor);
    }

    /// The core run loop of the engine. This function will spawn a thread for
    /// each collector, strategy, and executor. It will then orchestrate the
    /// data flow between them.
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
                                ERROR_COUNT
                                    .with_label_values(&["executor", "execution_error"])
                                    .inc();
                                error!(
                                    error = %BotError::executor_error_with_source("Failed to execute action", e),
                                    "executor error"
                                );
                            } else {
                                let duration = start.elapsed().as_secs_f64();
                                ACTION_EXECUTION_DURATION
                                    .with_label_values(&[&executor_label])
                                    .observe(duration);
                                ACTIONS_EXECUTED_TOTAL
                                    .with_label_values(&[&executor_label])
                                    .inc();
                            }
                        }
                        Err(e) => {
                            ERROR_COUNT
                                .with_label_values(&["executor", "channel_error"])
                                .inc();
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
                ERROR_COUNT
                    .with_label_values(&["strategy", "sync_error"])
                    .inc();
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
                            
                            EVENT_PROCESSING_DURATION
                                .with_label_values(&[&strategy_label])
                                .observe(duration);

                            for action in actions {
                                if let Err(e) = action_sender.send(action) {
                                    ERROR_COUNT
                                        .with_label_values(&["strategy", "channel_error"])
                                        .inc();
                                    error!(
                                        error = %BotError::channel_error(format!("Failed to send action: {}", e)),
                                        "channel error"
                                    );
                                }
                            }

                            // Update queue size metrics
                            ACTION_QUEUE_SIZE
                                .with_label_values(&[&strategy_label])
                                .set(action_sender.len() as i64);
                        }
                        Err(e) => {
                            ERROR_COUNT
                                .with_label_values(&["strategy", "channel_error"])
                                .inc();
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
                        ERROR_COUNT
                            .with_label_values(&["collector", "stream_error"])
                            .inc();
                        error!(
                            error = %BotError::collector_error_with_source("Failed to get event stream", e),
                            "collector error"
                        );
                        return;
                    }
                };

                while let Some(event) = event_stream.next().await {
                    if let Err(e) = event_sender.send(event) {
                        ERROR_COUNT
                            .with_label_values(&["collector", "channel_error"])
                            .inc();
                        error!(
                            error = %BotError::channel_error(format!("Failed to send event: {}", e)),
                            "channel error"
                        );
                    } else {
                        EVENTS_PROCESSED_TOTAL
                            .with_label_values(&[&collector_label])
                            .inc();
                        
                        // Update queue size metrics
                        EVENT_QUEUE_SIZE
                            .with_label_values(&[&collector_label])
                            .set(event_sender.len() as i64);
                    }
                }
            });
        }

        Ok(set)
    }
}
