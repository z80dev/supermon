use async_trait::async_trait;
use tokio::sync::mpsc::Sender;

#[async_trait]
pub trait Watcher {
    type Payload;
    async fn watch(&self, sx: Sender<Self::Payload>);
}
