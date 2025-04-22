use async_trait::async_trait;
use botcore::{
    error::Result,
    types::{Collector, CollectorStream, Executor, Strategy},
};
use criterion::{criterion_group, criterion_main, Criterion};
use futures::stream::FuturesUnordered;
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
        // Pre-allocate events vector
        let mut events = Vec::with_capacity(self.num_events);
        events.extend((0..self.num_events).map(|i| BenchEvent(i as u64)));
        Ok(Box::pin(tokio_stream::iter(events)))
    }
}

// Fast strategy for benchmarking using atomic state
struct BenchStrategy {
    state: std::sync::atomic::AtomicU64,
}

impl BenchStrategy {
    fn new() -> Self {
        Self {
            state: std::sync::atomic::AtomicU64::new(0),
        }
    }
}

#[async_trait]
impl Strategy<BenchEvent, BenchAction> for BenchStrategy {
    async fn sync_state(&mut self) -> Result<()> {
        self.state.store(0, std::sync::atomic::Ordering::Relaxed);
        Ok(())
    }

    async fn process_event(&mut self, event: BenchEvent) -> Vec<BenchAction> {
        let new_state = self
            .state
            .fetch_add(event.0, std::sync::atomic::Ordering::Relaxed);
        vec![BenchAction(new_state + event.0)]
    }
}

// Fast executor for benchmarking
struct BenchExecutor {
    executed_actions: Arc<std::sync::atomic::AtomicU64>,
}

impl BenchExecutor {
    fn new() -> Self {
        Self {
            executed_actions: Arc::new(std::sync::atomic::AtomicU64::new(0)),
        }
    }
}

#[async_trait]
impl Executor<BenchAction> for BenchExecutor {
    async fn execute(&self, _action: BenchAction) -> Result<()> {
        self.executed_actions
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        Ok(())
    }
}

fn bench_event_processing(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    c.bench_function("event_processing", |b| {
        b.iter(|| {
            rt.block_on(async {
                let collector = BenchCollector { num_events: 1000 };
                let mut strategy = BenchStrategy::new();
                let executor = BenchExecutor::new();

                let mut event_stream = collector.get_event_stream().await.unwrap();
                while let Some(event) = tokio_stream::StreamExt::next(&mut event_stream).await {
                    let actions = strategy.process_event(event).await;
                    for action in actions {
                        executor.execute(action).await.unwrap();
                    }
                }

                assert_eq!(
                    executor
                        .executed_actions
                        .load(std::sync::atomic::Ordering::Relaxed),
                    1000
                );
            });
        });
    });
}

fn bench_concurrent_processing(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    c.bench_function("concurrent_processing", |b| {
        b.iter(|| {
            rt.block_on(async {
                let collector = BenchCollector { num_events: 1000 };
                let strategy = Arc::new(Mutex::new(BenchStrategy::new()));
                let executor = Arc::new(BenchExecutor::new());

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

                while (futures::StreamExt::next(&mut futures).await).is_some() {}

                assert_eq!(
                    executor
                        .executed_actions
                        .load(std::sync::atomic::Ordering::Relaxed),
                    1000
                );
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
                let collector = BenchCollector { num_events: 1000 };
                let mut strategy = BenchStrategy::new();
                let executor = BenchExecutor::new();

                let start = Instant::now();
                let mut event_stream = collector.get_event_stream().await.unwrap();

                while let Some(event) = tokio_stream::StreamExt::next(&mut event_stream).await {
                    let actions = strategy.process_event(event).await;
                    for action in actions {
                        executor.execute(action).await.unwrap();
                    }
                }

                let total_time = start.elapsed().as_nanos();
                assert!(total_time > 0);
                assert_eq!(
                    executor
                        .executed_actions
                        .load(std::sync::atomic::Ordering::Relaxed),
                    1000
                );
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
