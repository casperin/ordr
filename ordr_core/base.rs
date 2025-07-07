use std::{any::TypeId, pin::Pin, sync::Arc, time::Duration};

use serde_json::Value;

pub trait State: Clone + Send + Sync + 'static {}
impl<T> State for T where T: Clone + Send + Sync + 'static {}

pub trait NodeBuilder<S: State> {
    fn node() -> Node<S>;
}

#[derive(Clone)]
pub struct Node<S: State> {
    pub name: &'static str,
    pub id: TypeId,
    pub deps: Arc<dyn Fn() -> Vec<Node<S>> + Send + Sync + 'static>,
    pub producer: Producer<S>,
}

impl<S: State> std::fmt::Debug for Node<S> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Node<{}>", self.name)
    }
}

type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

pub type Producer<S> = Arc<
    dyn Fn(Context<S>, Vec<Value>) -> BoxFuture<'static, Result<Value, Error>>
        + Send
        + Sync
        + 'static,
>;

#[derive(Debug, Clone)]
pub struct Context<S: State> {
    pub state: S,
    pub retry: u32,
    pub start: Duration,
}

#[derive(Debug)]
pub struct Error {
    pub message: String,
    pub retry_in: Option<Duration>,
}
