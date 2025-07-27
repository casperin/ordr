use std::{any::TypeId, pin::Pin, sync::Arc, time::Duration};

use serde_json::Value;

/// Public because macros need it.
#[doc(hidden)]
pub trait State: Clone + Send + Sync + 'static {}
impl<T> State for T where T: Clone + Send + Sync + 'static {}

/// Trait for building a node. Users of [`ordr`] should not need to know this.
/// Public because macros need it.
#[doc(hidden)]
pub trait NodeBuilder<S: State> {
    fn node() -> Node<S>;
}

/// An actual node.
/// Public because macros need it.
#[doc(hidden)]
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

/// Public because macros need it.
#[doc(hidden)]
pub type Producer<S> = Arc<
    dyn Fn(Context<S>, Vec<Value>) -> BoxFuture<'static, Result<Value>> + Send + Sync + 'static,
>;

/// First argument of a producer function. It's just some basic meta data (that I might later
/// expand on) about running the node.
///
/// It also contains your state.
#[derive(Debug, Clone)]
pub struct Context<S: State> {
    /// Your state as passed into the [`crate::Job`].
    pub state: S,
    /// Retry count. First time this is run, it will be `0`.
    pub retry: u32,
    /// The start time for this node.
    /// All "times" are defined as an offset of when the job started.
    pub start: Duration,
}

/// Return value for producers.
pub type Result<T> = std::result::Result<T, Error>;

/// Error type that a producer may return.
#[derive(Debug, Clone)]
pub struct Error {
    pub(crate) message: String,
    pub(crate) retry_in: Option<Duration>,
}

impl Error {
    /// Node has failed, and should not be retried.
    pub fn fatal(message: impl Into<String>) -> Self {
        let message = message.into();
        let retry_in = None;
        Self { message, retry_in }
    }

    /// Node has failed and should be retried after some time.
    pub fn with_retry(message: impl Into<String>, retry_in: Duration) -> Self {
        let message = message.into();
        let retry_in = Some(retry_in);
        Self { message, retry_in }
    }
}

/// Output of running a job. Describes how and if the job was finished. Use [`crate::Worker::data`]
/// to get the results out.
#[derive(Debug, Clone)]
pub enum Output {
    /// Job finished successfully.
    Done {
        /// It took this long to finish the job.
        duration: Duration,
    },
    /// Jab finished because a node failed.
    NodeFailed {
        /// Node failed after this much time.
        duration: Duration,
        /// Name of the node that failed.
        name: &'static str,
        /// The error message from the node.
        error: String,
    },
    /// Job finished because a node panicked.
    NodePanic {
        /// The node panicked at this time.
        duration: Duration,
        /// Name of the node that panicked.
        name: &'static str,
        /// Error message of the node panicking.
        error: String,
    },
    /// Job was manually stopped.
    Stopped {
        /// Job was stopped after this time.
        duration: Duration,
    },
}

impl Output {
    /// Returns the total duration of running the job.
    #[must_use]
    pub fn duration(&self) -> Duration {
        match self {
            Output::Stopped { duration }
            | Output::NodePanic { duration, .. }
            | Output::NodeFailed { duration, .. }
            | Output::Done { duration } => *duration,
        }
    }
    #[must_use]
    pub fn is_done(&self) -> bool {
        matches!(self, Self::Done { .. })
    }
    #[must_use]
    pub fn is_stopped(&self) -> bool {
        matches!(self, Self::Stopped { .. })
    }
    #[must_use]
    pub fn is_node_failed(&self) -> bool {
        matches!(self, Self::NodeFailed { .. })
    }
    #[must_use]
    pub fn is_node_panic(&self) -> bool {
        matches!(self, Self::NodePanic { .. })
    }
}
