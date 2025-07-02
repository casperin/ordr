use std::{
    collections::{HashMap, HashSet},
    fmt::Display,
};

use tokio::{select, task::JoinSet};
use tracing::{error, info};

use crate::{
    error::Error,
    job::Job,
    mermaid::mermaid,
    node::{Node, NodeBuilder, Payload},
    outputs::Outputs,
    validation::{validate_job, validate_nodes},
};

/// Context for nodes.
pub trait Ctx: Clone + Send + 'static {}
impl<T> Ctx for T where T: Clone + Send + 'static {}

/// Error type for nodes. All nodes in a graph must return the same error type.
pub trait Er: Send + 'static + Display {}
impl<T> Er for T where T: Send + 'static + Display {}

/// Creates a graph from a static set of Nodes.
///
/// # Example
/// ```ignore
/// #[derive(Clone)]
/// struct A;
///
/// #[derive(Clone)]
/// struct B;
///
/// let graph = create_graph!(A, B).unwrap();
/// ```
#[macro_export]
macro_rules! build {
    ( $( $ty:ty ),* $(,)? ) => {{
        let mut graph = $crate::graph::Graph::builder();
        $(
            graph.add_node::<$ty>();
        )*
        graph.build()
    }};
}

/// Graph builder. Call [`Builder::build`] when you are done adding nodes.
pub struct Builder<C: Ctx, E: Er> {
    nodes: Vec<Node<C, E>>,
}

impl<C: Ctx, E: Er> Builder<C, E> {
    /// Add a node to the graph.
    pub fn add_node<T: NodeBuilder<C, E>>(&mut self) {
        self.nodes.push(T::node());
    }

    /// Add a node to the graph.
    #[must_use]
    pub fn with_node<T: NodeBuilder<C, E>>(mut self) -> Self {
        self.add_node::<T>();
        self
    }

    /// Validates and builds the [`Graph`], so it's ready for use.
    /// # Errors
    /// If the graph contains cycles, or there are missing dependencies, etc.
    pub fn build(self) -> Result<Graph<C, E>, Error<E>> {
        let mut nodes = self.nodes;
        // Sort by ID so we can do binary searches when
        // (rarely) we need to look up a node by id.
        nodes.sort_by_key(|n| n.id);
        // Filter out repeated nodes.
        let mut seen = HashSet::new();
        nodes.retain(|node| seen.insert(node.id));
        // Validate that nodes don't contain cycles, all
        // deps are in the graph, etc.
        let adj = validate_nodes(&nodes)?;
        Ok(Graph { nodes, adj })
    }
}

/// Main struct of this crate. Holds a list of all nodes and keeps track of dependencies between them.
pub struct Graph<C: Ctx, E: Er> {
    /// List of all nodes in the graph.
    pub(crate) nodes: Vec<Node<C, E>>,
    /// Adjencency list.
    pub(crate) adj: Vec<Vec<usize>>,
}

impl<C: Ctx, E: Er> Graph<C, E> {
    /// Create a builder.
    #[must_use]
    pub fn builder() -> Builder<C, E> {
        Builder { nodes: vec![] }
    }

    /// Retrieve the name of a node at the index.
    /// # Panics
    /// If index is too large.
    #[must_use]
    pub fn node_name(&self, i: usize) -> &'static str {
        self.nodes[i].name
    }

    /// Validates and executes a [`Job`].
    /// # Errors
    /// If the [`Job`] is not valid with regards to the graph (say, a target is not present), or
    /// if any of the nodes fail during execution.
    pub fn validate_job(&self, job: &Job<C, E>) -> Result<(), Error<E>> {
        validate_job(&self.nodes, job)
    }

    fn outputs(&self, results: HashMap<usize, Payload>) -> Outputs {
        let id_to_payload = results
            .into_iter()
            .map(|(i, payload)| (self.nodes[i].id, payload))
            .collect();
        Outputs::new(id_to_payload)
    }

    /// Creates a mermaid diagram of the executtion of this job.
    #[must_use]
    pub fn mermaid(&self, job: &Job<C, E>) -> String {
        mermaid(self, job)
    }

    /// Validates and executes a [`Job`].
    /// # Errors
    /// If the [`Job`] is not valid with regards to the graph (say, a target is not present), or
    /// if any of the nodes fail during execution.
    pub async fn execute(&self, job: Job<C, E>, ctx: C) -> Result<Outputs, Error<E>> {
        let mut handles = JoinSet::new();
        let mut pending = job.pending(self);
        let mut results = job
            .inputs
            .into_iter()
            .filter_map(|(id, payload)| {
                let i = self.nodes.binary_search_by_key(&id, |n| n.id).ok()?;
                Some((i, payload))
            })
            .collect::<HashMap<_, _>>();

        info!(count = pending.len(), "Job start");

        loop {
            // Find all the ready nodes (those whose deps are done).
            let is_done = |i| results.contains_key(i);
            let ready = pending.extract_if(|i| self.adj[*i].iter().all(is_done));

            // Start the ready nodes.
            for i in ready {
                let node = &self.nodes[i];
                let payloads = self.adj[i].iter().map(|i| &results[i]).collect();
                let payload = (node.prepare)(payloads);
                let execute = (node.execute).clone();
                let ctx = ctx.clone();
                info!(node = node.name, "Node start");
                handles.spawn(async move { (i, execute(ctx, payload).await) });
            }

            select! {
                res = handles.join_next() => {
                    match res {
                        Some(Ok((i, Ok(r)))) => {
                            info!(node = self.node_name(i), "Node done");
                            results.insert(i, r);
                        }
                        Some(Ok((i, Err(error)))) => {
                            error!(node = self.node_name(i), error = error.to_string(), "Node failed");
                            let outputs = self.outputs(results);
                            return Err(Error::NodeFailed { outputs, i, error });
                        }
                        Some(Err(error)) => {
                            error!(?error, "Node panicked");
                            let outputs = self.outputs(results);
                            return Err(Error::NodePanic { outputs, error });
                        }
                        None => {
                            info!("Job done");
                            let outputs = self.outputs(results);
                            return Ok(outputs);
                        }
                    }
                }
                () = job.cancellation_token.cancelled() => {
                    info!("Job cancelled");
                    let outputs = self.outputs(results);
                    return Err(Error::Cancelled { outputs });
                }
            }
        }
    }
}
