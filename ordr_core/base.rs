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
    pub(crate) message: String,
    pub(crate) retry_in: Option<Duration>,
}

impl Error {
    pub fn fatal(message: impl Into<String>) -> Self {
        let message = message.into();
        let retry_in = None;
        Self { message, retry_in }
    }

    pub fn with_retry(message: impl Into<String>, retry_in: Duration) -> Self {
        let message = message.into();
        let retry_in = Some(retry_in);
        Self { message, retry_in }
    }
}

#[derive(Debug, Clone)]
pub enum Output {
    Done(Duration),
    NodeFailed(Duration, &'static str, String),
    NodePanic(Duration, &'static str, String),
    Stopped(Duration),
}

impl Output {
    /// Returns the total duration of running the job.
    pub fn duration(&self) -> Duration {
        match self {
            Output::Done(duration) => *duration,
            Output::NodeFailed(duration, _, _) => *duration,
            Output::NodePanic(duration, _, _) => *duration,
            Output::Stopped(duration) => *duration,
        }
    }
    pub fn is_done(&self) -> bool {
        matches!(self, Self::Done(..))
    }
    pub fn is_stopped(&self) -> bool {
        matches!(self, Self::Stopped(..))
    }
    pub fn is_node_failed(&self) -> bool {
        matches!(self, Self::NodeFailed(..))
    }
    pub fn is_node_panic(&self) -> bool {
        matches!(self, Self::NodePanic(..))
    }
}
