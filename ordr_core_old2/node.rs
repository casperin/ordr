use std::{any::TypeId, pin::Pin, sync::Arc, time::Duration};

type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

pub type Producer<S> = Arc<
    dyn Fn(Context<S>, Vec<Vec<u8>>) -> BoxFuture<'static, Result<Vec<u8>, Error>>
        + Send
        + Sync
        + 'static,
>;

pub trait State: Clone + Send + 'static {}
impl<T> State for T where T: Clone + Send + 'static {}

pub struct Error {
    pub message: String,
    pub retry_in: Option<Duration>,
}

pub struct Context<S: State> {
    pub state: S,
    pub retry: u32,
    pub start: Duration,
}

pub trait Node<S: State> {
    fn id() -> TypeId;
    fn name() -> &'static str;
    fn deps() -> Vec<TypeId>;
    fn producer() -> Producer<S>;
}
