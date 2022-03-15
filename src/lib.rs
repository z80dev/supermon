mod consumer;
mod executor;
mod middleware;
mod watcher;

pub use consumer::Consumer;
pub use executor::Executor;
pub use middleware::Middleware;
pub use watcher::Watcher;

#[cfg(test)]
mod tests {
    use async_trait::async_trait;
    use crate::executor::Executor;

    #[derive(Debug, Clone, Copy, Default)]
    struct TestWorker { start_val: u32, delta: u32, end_val: u32 }

    #[async_trait]
    impl crate::Watcher for TestWorker {
        type Payload = u32;

        async fn watch(&self, sx: tokio::sync::mpsc::Sender<u32>) {
            sx.send(self.start_val).await.expect("failed to send number through channel");
        }
    }

    #[async_trait]
    impl crate::Middleware for TestWorker {
        type Payload = u32;
        async fn process(&self, sx: tokio::sync::mpsc::Sender<Self::Payload>, mut rx: tokio::sync::mpsc::Receiver<Self::Payload>) {
            if let Some(n) = rx.recv().await {
                sx.send(n + self.delta).await.expect("failed to send number through channel");
            }
        }
    }

    #[async_trait]
    impl crate::Consumer for TestWorker {
        type Payload = u32;
        async fn consume(&self, mut rx: tokio::sync::mpsc::Receiver<Self::Payload>) {
            if let Some(n) = rx.recv().await {
                assert!(n == self.end_val, "Did not consume expected value");
            }
        }
        async fn consume_broadcast(&self, mut rx: tokio::sync::broadcast::Receiver<Self::Payload>) {
            if let Ok(n) = rx.recv().await {
                assert!(n == self.end_val, "Did not consume expected value");
            }
        }
    }

    #[tokio::test]
    async fn single_modules() {
        let mut executor: Executor<u32> = Executor::new();
        let worker = TestWorker{ start_val: 3, delta: 6, end_val: 9};
        assert!(worker.start_val + worker.delta == worker.end_val, "Faulty input test value");
        executor.add_watcher(Box::new(worker.clone()));
        executor.add_middleware(Box::new(worker.clone()));
        executor.add_listener(Box::new(worker.clone()));
        executor.start().await.expect("Failed to start executor");
    }
}
