use crate::consumer::Consumer;
use crate::middleware::Middleware;
use crate::watcher::Watcher;
use std::fmt::Debug;
use tokio::sync::broadcast;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;

pub struct Executor<T: Send + Sync + Clone + Debug + 'static> {
    watchers: Vec<Box<dyn Watcher<Payload = T> + Send + Sync>>,
    middlewares: Vec<Box<dyn Middleware<Payload = T> + Send + Sync>>,
    consumer: Option<Box<dyn Consumer<Payload = T> + Send + Sync>>,
    consumers: Vec<Box<dyn Consumer<Payload = T> + Send + Sync>>,
}

impl<T> Executor<T>
where
    T: Send + Sync + Clone + Debug + 'static,
{
    pub fn new() -> Executor<T> {
        return Executor {
            watchers: vec![],
            middlewares: vec![],
            consumer: None,
            consumers: vec![],
        };
    }

    pub fn add_watcher(&mut self, w: Box<dyn Watcher<Payload = T> + Send + Sync>) {
        self.watchers.push(w);
    }

    pub fn set_listener(&mut self, l: Box<dyn Consumer<Payload = T> + Send + Sync>) {
        self.consumer = Some(l);
    }

    pub fn add_listener(&mut self, l: Box<dyn Consumer<Payload = T> + Send + Sync>) {
        self.consumers.push(l);
    }

    pub fn add_middleware(&mut self, m: Box<dyn Middleware<Payload = T> + Send + Sync>) {
        self.middlewares.push(m);
    }

    pub async fn start(self) {
        // create first channel
        let (sx, rx): (mpsc::Sender<T>, mpsc::Receiver<T>) = mpsc::channel(32);

        // create vector of JoinHandles to be awaited later
        let mut handles: Vec<JoinHandle<()>> = Vec::new();

        for w in self.watchers {
            /*
             * Each Watcher expects a Sender channel
             * They will send any "found" values through this channel
             * Each gets its own clone, all feed into the same Receiver
             */
            let sxw = sx.clone();
            let handle = tokio::spawn(async move {
                w.watch(sxw).await;
            });
            handles.push(handle);
        }

        // All watcher values received on this channel
        let mut watchers_receiver = rx;

        for mw in self.middlewares.into_iter() {
            /*
             * Each middleware expects a Sender and Receiver
             * We create a new Sender for each one and pipe them together
             */
            let (sxn, rxn): (mpsc::Sender<T>, mpsc::Receiver<T>) = mpsc::channel(10);
            let mw_handle = tokio::spawn(async move {
                mw.process(sxn, watchers_receiver).await;
            });
            watchers_receiver = rxn;
            handles.push(mw_handle);
        }

        if !self.consumers.is_empty() {
            // Now we need to switch to a broadcast channel, to support multiple Consumers
            // We need to pipe the middleware output to a broadcast Sender

            let (bsx, _): (broadcast::Sender<T>, broadcast::Receiver<T>) = broadcast::channel(16);

            let bsx_pipe_input = bsx.clone();
            let mw_to_consumers_handle = tokio::spawn(async move {
                while let Some(r) = watchers_receiver.recv().await {
                    bsx_pipe_input.send(r).unwrap();
                }
            });

            handles.push(mw_to_consumers_handle);

            // Then clone receivers and pass one to each Consumer
            for l in self.consumers {
                let brx = bsx.subscribe();
                let listener = tokio::spawn(async move {
                    l.consume_broadcast(brx).await;
                });
                handles.push(listener);
            }
        } else {
            // single consumer, stick with mpsc
            if let Some(c) = self.consumer {
                let listener = tokio::spawn(async move {
                    c.consume(watchers_receiver).await;
                });
                listener.await.unwrap();
            }
        }
        for handle in handles {
            handle.await.unwrap();
        }
    }
}
