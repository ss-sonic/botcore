use async_trait::async_trait;
use botcore::{
    error::Result,
    types::{Collector, CollectorStream, Executor, Strategy},
};
use criterion::{criterion_group, criterion_main, Criterion};
use std::sync::Arc;
use tokio::runtime::Runtime;
use tokio::sync::Mutex;

// Test types
#[derive(Debug, Clone, PartialEq)]
struct BenchEvent(u64);

#[derive(Debug, Clone, PartialEq)]
struct BenchAction(u64);

// High-throughput collector for benchmarking
struct BenchCollector {
    num_events: usize,
}

#[async_trait]
impl Collector<BenchEvent> for BenchCollector {
    async fn get_event_stream(&self) -> Result<CollectorStream<'_, BenchEvent>> {
        let events = (0..self.num_events).map(|i| BenchEvent(i as u64));
        Ok(Box::pin(tokio_stream::iter(events)))
    }
}

// Fast strategy for benchmarking
struct BenchStrategy {
    state: Arc<Mutex<u64>>,
}

#[async_trait]
impl Strategy<BenchEvent, BenchAction> for BenchStrategy {
    async fn sync_state(&mut self) -> Result<()> {
        let mut state = self.state.lock().await;
        *state = 0;
        Ok(())
    }

    async fn process_event(&mut self, event: BenchEvent) -> Vec<BenchAction> {
        let mut state = self.state.lock().await;
        *state += event.0;
        vec![BenchAction(*state)]
    }
}

// Fast executor for benchmarking
struct BenchExecutor {
    executed_actions: Arc<Mutex<Vec<BenchAction>>>,
}

#[async_trait]
impl Executor<BenchAction> for BenchExecutor {
    async fn execute(&self, action: BenchAction) -> Result<()> {
        let mut actions = self.executed_actions.lock().await;
        actions.push(action);
        Ok(())
    }
}

fn bench_event_processing(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    c.bench_function("event_processing", |b| {
        b.iter(|| {
            rt.block_on(async {
                // Set up benchmark components
                let collector = BenchCollector { num_events: 1000 };
                let state = Arc::new(Mutex::new(0));
                let mut strategy = BenchStrategy {
                    state: Arc::clone(&state),
                };
                let executed_actions = Arc::new(Mutex::new(Vec::new()));
                let executor = BenchExecutor {
                    executed_actions: Arc::clone(&executed_actions),
                };

                // Process events through the pipeline
                let mut event_stream = collector.get_event_stream().await.unwrap();
                while let Some(event) = tokio_stream::StreamExt::next(&mut event_stream).await {
                    let actions = strategy.process_event(event).await;
                    for action in actions {
                        executor.execute(action).await.unwrap();
                    }
                }

                // Verify results
                let actions = executed_actions.lock().await;
                assert_eq!(actions.len(), 1000);
            });
        });
    });
}

fn bench_concurrent_processing(c: &mut Criterion) {
    use futures::stream::FuturesUnordered;

    let rt = Runtime::new().unwrap();

    c.bench_function("concurrent_processing", |b| {
        b.iter(|| {
            rt.block_on(async {
                // Set up benchmark components
                let collector = BenchCollector { num_events: 1000 };
                let state = Arc::new(Mutex::new(0));
                let strategy = Arc::new(Mutex::new(BenchStrategy {
                    state: Arc::clone(&state),
                }));
                let executed_actions = Arc::new(Mutex::new(Vec::new()));
                let executor = Arc::new(BenchExecutor {
                    executed_actions: Arc::clone(&executed_actions),
                });

                // Process events concurrently
                let mut event_stream = collector.get_event_stream().await.unwrap();
                let mut futures = FuturesUnordered::new();

                while let Some(event) = tokio_stream::StreamExt::next(&mut event_stream).await {
                    let strategy = Arc::clone(&strategy);
                    let executor = Arc::clone(&executor);

                    futures.push(async move {
                        let mut strategy = strategy.lock().await;
                        let actions = strategy.process_event(event).await;
                        for action in actions {
                            executor.execute(action).await.unwrap();
                        }
                    });
                }

                // Wait for all processing to complete
                while let Some(_) = futures::StreamExt::next(&mut futures).await {}

                // Verify results
                let actions = executed_actions.lock().await;
                assert_eq!(actions.len(), 1000);
            });
        });
    });
}

fn bench_metrics_collection(c: &mut Criterion) {
    use std::time::Instant;

    let rt = Runtime::new().unwrap();

    c.bench_function("metrics_collection", |b| {
        b.iter(|| {
            rt.block_on(async {
                // Set up benchmark components with metrics tracking
                let collector = BenchCollector { num_events: 1000 };
                let state = Arc::new(Mutex::new(0));
                let mut strategy = BenchStrategy {
                    state: Arc::clone(&state),
                };
                let executed_actions = Arc::new(Mutex::new(Vec::new()));
                let executor = BenchExecutor {
                    executed_actions: Arc::clone(&executed_actions),
                };

                // Process events while tracking metrics
                let mut event_stream = collector.get_event_stream().await.unwrap();
                let mut total_processing_time = 0u128;
                let mut event_count = 0;

                while let Some(event) = tokio_stream::StreamExt::next(&mut event_stream).await {
                    let start = Instant::now();

                    let actions = strategy.process_event(event).await;
                    for action in actions {
                        executor.execute(action).await.unwrap();
                    }

                    total_processing_time += start.elapsed().as_nanos();
                    event_count += 1;
                }

                // Calculate and verify metrics
                let avg_processing_time = total_processing_time / event_count as u128;
                assert!(avg_processing_time > 0);
                assert_eq!(event_count, 1000);
            });
        });
    });
}

criterion_group!(
    benches,
    bench_event_processing,
    bench_concurrent_processing,
    bench_metrics_collection
);
criterion_main!(benches);
