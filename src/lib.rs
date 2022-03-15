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

    #[derive(Debug, Default)]
    struct TestWorker {}

    #[async_trait]
    impl crate::Watcher for TestWorker {
        type Payload = u32;

        async fn watch(&self, sx: tokio::sync::mpsc::Sender<u32>) {
            sx.send(1u32).await.expect("failed to send number through channel");
        }
    }

    #[async_trait]
    impl crate::Middleware for TestWorker {
        type Payload = u32;
        async fn process(&self, sx: tokio::sync::mpsc::Sender<Self::Payload>, mut rx: tokio::sync::mpsc::Receiver<Self::Payload>) {
            if let Some(n) = rx.recv().await {
                sx.send(n + 1_u32).await.expect("failed to send number through channel");
            }
        }
    }

    #[async_trait]
    impl crate::Consumer for TestWorker {
        type Payload = u32;
        async fn consume(&self, mut rx: tokio::sync::mpsc::Receiver<Self::Payload>) {
            if let Some(n) = rx.recv().await {
                assert!(n == 2, "Did not consume expected value");
            }
        }
        async fn consume_broadcast(&self, mut rx: tokio::sync::broadcast::Receiver<Self::Payload>) {
            if let Ok(n) = rx.recv().await {
                assert!(n == 2, "Did not consume expected value");
            }
        }
    }

    #[tokio::test]
    async fn it_works() {
        let mut executor: Executor<u32> = Executor::new();
        executor.add_watcher(Box::new(TestWorker{}));
        executor.add_middleware(Box::new(TestWorker{}));
        executor.add_listener(Box::new(TestWorker{}));
        executor.start().await.expect("Failed to start executor");
    }
}
