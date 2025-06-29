use std::{
    any::TypeId,
    collections::{HashMap, HashSet},
    marker::PhantomData,
};

use tokio_util::sync::CancellationToken;

use crate::{
    graph::{Ctx, Er, Graph},
    node::{NodeBuilder, Payload},
};

/// Describes a set of work to be executed by a [`crate::graph::Graph`].
pub struct Job<C: Ctx, E: Er> {
    /// Execution will solve the graph for reaching (and executing) these nodes.
    pub(crate) targets: HashSet<TypeId>,
    /// Optionally provided values. Used for skipping parts of a graph.
    pub(crate) inputs: HashMap<TypeId, Payload>,
    /// Cancellation token for stopping execution of a job. The default token never cancels.
    pub(crate) cancellation_token: CancellationToken,
    /// Types of nodes. Used for compile time guarantee that the job fits the graph.
    node_type: PhantomData<(C, E)>,
}

impl<C: Ctx, E: Er> Job<C, E> {
    /// Create a new empty Job.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets a provided [`CancellationToken`] if you already have one.
    ///
    /// The more common usecase here, though, is to clone the one already present and call cancel on that. See [`Job::cancellation_token`].
    #[must_use]
    pub fn with_cancellation_token(mut self, cancellation_token: CancellationToken) -> Self {
        self.cancellation_token = cancellation_token;
        self
    }

    /// Clone the [`CancellationToken`]. Useful for, eg. implementing a timeout for the job.
    #[must_use]
    pub fn cancellation_token(&self) -> CancellationToken {
        self.cancellation_token.clone()
    }

    /// Set a target for the job. Without setting this, the job is essentially a no-op.
    /// # Example
    /// ```ignore
    /// #[derive(Clone)]
    /// struct A;
    ///
    /// #[executor]
    /// async fn create_a(_: ()) -> Result<A, String> {
    ///     Ok(A)
    /// }
    ///
    /// let mut job = Job::new();
    /// job.target::<A>();
    /// ```
    pub fn target<T: 'static + NodeBuilder<C, E>>(&mut self) {
        self.targets.insert(TypeId::of::<T>());
    }

    /// Set a target for the job. Without setting this, the job is essentially a no-op.
    ///
    /// # Example
    /// ```ignore
    /// #[derive(Clone)]
    /// struct A;
    ///
    /// #[executor]
    /// async fn create_a(_: ()) -> Result<A, String> {
    ///     Ok(A)
    /// }
    ///
    /// let job = Job::new().with_target::<A>();
    /// ```
    #[must_use]
    pub fn with_target<T: 'static + NodeBuilder<C, E>>(mut self) -> Self {
        self.target::<T>();
        self
    }

    /// Adds data to the job that will be used as if the node producing that data has already run,
    /// and none of its dependencies will be run either. This is useful for things like cached
    /// values, or continuing a previously failed execution.
    ///
    /// # Example
    /// ```ignore
    /// #[derive(Clone)]
    /// struct A(i32);
    ///
    /// let mut job = Job::new();
    /// job.with_input(A(22));
    /// ```
    pub fn input<T: Send + 'static>(&mut self, value: T) {
        let key = TypeId::of::<T>();
        let val = Box::new(value) as Payload;
        self.inputs.insert(key, val);
    }

    /// Adds data to the job that will be used as if the node producing that data has already run,
    /// and none of its dependencies will be run either. This is useful for things like cached
    /// values, or continuing a previously failed execution.
    ///
    /// # Example
    /// ```ignore
    /// #[derive(Clone)]
    /// struct A(i32);
    ///
    /// let job = Job::new().with_input(A(22));
    /// ```
    #[must_use]
    pub fn with_input<T: Send + 'static>(mut self, value: T) -> Self {
        self.input(value);
        self
    }

    pub(crate) fn pending(&self, graph: &Graph<C, E>) -> HashSet<usize> {
        let mut pending = HashSet::new();
        let mut stack: Vec<usize> = vec![];
        for id in &self.targets {
            if !self.inputs.contains_key(id) {
                let i = graph.nodes.binary_search_by_key(id, |n| n.id).unwrap();
                pending.insert(i);
                stack.extend(&graph.adj[i]);
            }
        }
        while let Some(i) = stack.pop() {
            let id = graph.nodes[i].id;
            let not_input = !self.inputs.contains_key(&id);
            let not_pending = !pending.contains(&i);
            if not_input && not_pending {
                pending.insert(i);
                stack.extend(&graph.adj[i]);
            }
        }
        pending
    }
}

impl<C: Ctx, E: Er> Default for Job<C, E> {
    fn default() -> Self {
        Self {
            targets: HashSet::new(),
            inputs: HashMap::new(),
            cancellation_token: CancellationToken::new(),
            node_type: PhantomData,
        }
    }
}
