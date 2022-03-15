use async_trait::async_trait;
use tokio::sync::broadcast;
use tokio::sync::mpsc::Receiver;

#[async_trait]
pub trait Consumer {
    type Payload;
    async fn consume(&self, mut rx: Receiver<Self::Payload>);
    async fn consume_broadcast(&self, mut rx: broadcast::Receiver<Self::Payload>);
}
