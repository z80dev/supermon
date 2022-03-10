# Supermon
A tool for scheduling watchers and feeding their results through middleware to a consumer

Supermon lets you write standalone units of computation (workers) and handles wiring the communication between them.
It uses channels to communicate messages between these different units.

## How It Works

There are three different types of workers currently supported

- Watchers: Watch for certain conditions, and send a message through a channel if it finds anything
- Middleware: Processes messages between watchers and listeners in order to perform necessary processing such as deduplication
- Consumers: Receive messages originating from watchers, and carry out any necessary actions.

This is a really flexible base upon which you can build anything tailored to your needs.

For each of these roles, there is a corresponding trait. You can implement these traits on any struct in order for Supermon to schedule its execution.

### Watcher

``` rust
// watcher.rs
use async_trait::async_trait;
use tokio::sync::mpsc::Sender;

#[async_trait]
pub trait Watcher {
    type Payload;
    async fn watch(&self, sx: Sender<Self::Payload>);
}
```

The `Watcher` trait has only one function that must be implemented, `watch`. This async function should send a message through the channel `sx` if it finds anything.

For example (pseudo-code, the `check_bal` function here is imaginary and just checks for a balance somewhere):

``` rust
pub struct SuperWatcher {
    pub addr_to_watch: String,
}

impl Watcher for MulticallZapperWatcher {
    type Payload = String;
    async fn watch(&self, sx: Sender<Self::Payload>) {
        loop {
            if check_bal(self.addr_to_watch) != 0 {
                sx.send(self.addr_to_watch);
            }
        }
    }
}
```

### Middleware

``` rust
use async_trait::async_trait;
use tokio::sync::mpsc::{Receiver, Sender};

#[async_trait]
pub trait Middleware {
    type Payload;
    async fn process(&self, sx: Sender<Self::Payload>, rx: Receiver<Self::Payload>);
}
```

The `Middleware` trait has only one function that must be implemented, `process`. This async function should listen for message from `rx`, perform any necessary processing or filtering, and pass messages along to `sx`.

This can be used to implement deduplication of messages.

### Consumer

``` rust
use async_trait::async_trait;
use tokio::sync::mpsc::Receiver;

#[async_trait]
pub trait Consumer {
    type Payload;
    async fn consume(&self, mut rx: Receiver<Self::Payload>);
}

```

The `Consumer` trait has only one function that must be implemented, `consume`. This async function should listen for messages on `rx` and perform any necessary actions.

For example, a consumer that logs any messages it receives:

``` rust
pub struct ListenerLogger{}

#[async_trait]
impl Consumer for ListenerLogger {
    type Payload = String;
    async fn consume(&self, mut rx: Receiver<Self::Payload>) {
        println!("Starting listener");
        while let Some(addr) = rx.recv().await {
            println!("Received address {} in message", addr);
        }
    }
}
```
