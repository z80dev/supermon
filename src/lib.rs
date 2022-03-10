mod executor;
mod consumer;
mod watcher;
mod middleware;

pub use executor::Executor;
pub use consumer::Consumer;
pub use watcher::Watcher;
pub use middleware::Middleware;

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        let result = 2 + 2;
        assert_eq!(result, 4);
    }
}
