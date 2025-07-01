use std::{
    any::{Any, TypeId},
    pin::Pin,
    sync::Arc,
};

/// Arbitrary sendable object that lives on the heap and can be sent.
pub(crate) type Payload = Box<dyn Any + Send>;

/// Output of the execute function on a [`Node`].
type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

/// Struct describing everything needed to find dependencies and execute this node.
#[allow(clippy::type_complexity)]
pub struct Node<C, E> {
    /// Name of this node. Only used for display.
    pub name: &'static str,

    /// The ID of this Node.
    pub id: TypeId,

    /// List of dependencies.
    pub deps: Vec<TypeId>,

    #[doc(hidden)]
    pub prepare: Arc<dyn Fn(Vec<&Payload>) -> Payload + Send + Sync + 'static>,

    #[doc(hidden)]
    pub execute:
        Arc<dyn Fn(C, Payload) -> BoxFuture<'static, Result<Payload, E>> + Send + Sync + 'static>,
}

/// Trait for creating a [`Node`] from an executor function.
///
/// Should be implemented automatically using the `executor` macro.
pub trait NodeBuilder<C, E> {
    /// Create node.
    fn node() -> Node<C, E>;
}
