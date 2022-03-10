use crate::consumer::Consumer;
use crate::watcher::Watcher;
use crate::middleware::Middleware;
use tokio::sync::mpsc;
use tokio::sync::mpsc::{Receiver, Sender};
use tokio::task::JoinHandle;

pub struct Executor<T: Send + Sync> {
    watchers: Vec<Box<dyn Watcher<Payload = T> + Send + Sync >>,
    middlewares: Vec<Box<dyn Middleware<Payload = T> + Send + Sync>>,
    consumer: Option<Box<dyn Consumer<Payload = T> + Send + Sync>>,
}

impl<T> Executor<T> where T: Send + Sync + 'static {
    pub fn new() -> Executor<T> {
        return Executor{ watchers: vec![], middlewares: vec![], consumer: None }
    }

    pub fn add_watcher(& mut self, w: Box<dyn Watcher<Payload = T> + Send + Sync >) {
       self.watchers.push(w);
    }

    pub fn set_listener(& mut self, l: Box<dyn Consumer<Payload = T> + Send + Sync>) {
        self.consumer = Some(l);
    }

    pub fn add_middleware(& mut self, m: Box<dyn Middleware<Payload = T> + Send + Sync>) {
       self.middlewares.push(m);
    }

    pub async fn start(self) {
        let (sx, rx): (Sender<T>, Receiver<T>) = mpsc::channel(32);
        let mut handles: Vec<JoinHandle<()>> = Vec::new();
        for w in self.watchers {
            let sxw = sx.clone();
            let handle = tokio::spawn(async move {
                w.watch(sxw).await;
            });
            handles.push(handle);
        }

        let mut watchers_receiver = rx;

        for mw in self.middlewares.into_iter() {
            let (sxn, rxn): (Sender<T>, Receiver<T>) = mpsc::channel(10);
            let mw_handle = tokio::spawn(async move {
                mw.process(sxn, watchers_receiver).await;
            });
            watchers_receiver = rxn;
            handles.push(mw_handle);
        }

        if let Some(c) = self.consumer {
            let listener = tokio::spawn(async move {
                c.consume(watchers_receiver).await;
            });
            listener.await.unwrap();
        }
        for handle in handles {
            handle.await.unwrap();
        }
    }
}
