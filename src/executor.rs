use crate::consumer::Consumer;
use crate::middleware::Middleware;
use crate::watcher::Watcher;
use std::error::Error;
use std::fmt::Debug;
use std::marker::Copy;
use tokio::sync::broadcast;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;

pub struct Executor<T: Send + Sync + Clone + Copy + Debug + 'static> {
    watchers: Vec<Box<dyn Watcher<Payload = T> + Send + Sync>>,
    middlewares: Vec<Box<dyn Middleware<Payload = T> + Send + Sync>>,
    consumer: Option<Box<dyn Consumer<Payload = T> + Send + Sync>>,
    consumers: Vec<Box<dyn Consumer<Payload = T> + Send + Sync>>,
}

impl<T> Executor<T>
where
    T: Send + Sync + Clone + Debug + Copy + 'static,
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

    pub fn add_middleware(&mut self, m: Box<dyn Middleware<Payload = T> + Send + Sync>) {
        self.middlewares.push(m);
    }

    pub fn add_listener(&mut self, l: Box<dyn Consumer<Payload = T> + Send + Sync>) {
        self.consumers.push(l);
    }

    pub fn set_listener(&mut self, l: Box<dyn Consumer<Payload = T> + Send + Sync>) {
        self.consumer = Some(l);
    }

    fn start_watchers(&mut self, sx: mpsc::Sender<T>) -> Vec<JoinHandle<()>> {
        start_watchers(&mut self.watchers, sx)
    }

    fn start_middlewares(&mut self, rx: mpsc::Receiver<T>) -> (Vec<JoinHandle<()>>, mpsc::Receiver<T>) {
        start_middlewares(&mut self.middlewares, rx)
    }

    fn start_consumers(&mut self, rx: mpsc::Receiver<T>) -> Vec<JoinHandle<()>> {
        start_consumers(&mut self.consumers, rx)
    }

    pub async fn start(mut self) -> Result<(), Box<dyn Error>> {
        // create first channel
        let (sx, rx): (mpsc::Sender<T>, mpsc::Receiver<T>) = mpsc::channel(32);

        // create vector of JoinHandles to be awaited later
        let mut handles: Vec<JoinHandle<()>> = Vec::new();

        handles.append(&mut self.start_watchers(sx));

        let (mut mw_handles, watchers_receiver) = self.start_middlewares(rx);
        handles.append(&mut mw_handles);

        if !self.consumers.is_empty() {
            handles.append(&mut self.start_consumers(watchers_receiver));
        } else {
            if let Some(l) = self.consumer {
                let listener = tokio::spawn(async move {
                    l.consume(watchers_receiver).await;
                });
                listener.await?;
            }
        }

        for handle in handles {
            handle.await?;
        }
        Ok(())
    }
}

fn start_watchers<T: Send + Sync + 'static>(
    watchers: &mut Vec<Box<dyn Watcher<Payload = T> + Send + Sync>>,
    sx: mpsc::Sender<T>,
) -> Vec<JoinHandle<()>> {
    let mut handles = vec![];

    // All watcher values received on this channel

    while let Some(mw) = watchers.pop() {
        /*
         * Each middleware expects a Sender and Receiver
         * We create a new Sender for each one and pipe them together
         */
        let sxn = sx.clone();
        let mw_handle = tokio::spawn(async move {
            mw.watch(sxn).await;
        });
        handles.push(mw_handle);
    }

    handles
}

fn start_middlewares<T: Send + Sync + 'static>(
    mws: &mut Vec<Box<dyn Middleware<Payload = T> + Send + Sync>>,
    rx: mpsc::Receiver<T>,
) -> (Vec<JoinHandle<()>>, mpsc::Receiver<T>) {
    let mut handles = vec![];

    // All watcher values received on this channel
    let mut watchers_receiver = rx;

    while let Some(mw) = mws.pop() {
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

    (handles, watchers_receiver)
}

fn start_consumers<T: Send + Sync + Clone + Debug + 'static>(
    consumers: &mut Vec<Box<dyn Consumer<Payload = T> + Send + Sync>>,
    mut rx: mpsc::Receiver<T>,
) -> Vec<JoinHandle<()>> {
    let mut handles = vec![];
    // Now we need to switch to a broadcast channel, to support multiple Consumers
    // We need to pipe the middleware output to a broadcast Sender

    let (bsx, _): (broadcast::Sender<T>, broadcast::Receiver<T>) = broadcast::channel(16);

    let bsx_pipe_input = bsx.clone();
    let mw_to_consumers_handle = tokio::spawn(async move {
        while let Some(r) = rx.recv().await {
            bsx_pipe_input
                .send(r)
                .expect("failed passing message from middlewares to consumers");
        }
    });

    handles.push(mw_to_consumers_handle);

    // Then clone receivers and pass one to each Consumer
    while let Some(l) = consumers.pop() {
        let brx = bsx.subscribe();
        let listener = tokio::spawn(async move {
            l.consume_broadcast(brx).await;
        });
        handles.push(listener);
    }
    handles
}
