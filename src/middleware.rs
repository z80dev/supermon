use async_trait::async_trait;
use tokio::sync::mpsc::{Receiver, Sender};

#[async_trait]
pub trait Middleware {
    type Payload;
    async fn process(&self, sx: Sender<Self::Payload>, rx: Receiver<Self::Payload>);
}
