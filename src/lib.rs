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
    #[test]
    fn it_works() {
        let result = 2 + 2;
        assert_eq!(result, 4);
    }
}
