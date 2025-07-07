use std::any::TypeId;

use tokio::task::JoinError;

use crate::{graph::Er, outputs::Outputs};

/// Describes anything that can go wrong.
#[derive(Debug)]
pub enum Error<E: Er> {
    /// Graph was built with no nodes. Use `graph.node::<MyNode>()` or `graph.add_node::<MyNode>()` to add nodes.
    NoNodes,
    /// The node wasn't found. The Job contained references to unknown nodes. Either as a target, or as some input.
    NodeNotFound(TypeId),
    /// A node is dependent on a node that isn't in the graph. The string is the name of the node
    /// having an unknown dependency, and the type id is that of the unknown node.
    DependencyNotFound(&'static str, TypeId),
    /// Your graph contains a cycle.
    Cycle(Vec<usize>),
    /// Execution of a job was cancelled from the outside.
    Cancelled {
        /// The outputs of the nodes already executed.
        outputs: Outputs,
    },
    /// Node ath this index panicked
    NodePanic {
        /// The outputs of the nodes already executed.
        outputs: Outputs,
        /// Error returned from tokio.
        error: JoinError,
    },
    /// A Node returned an error during execution, leading to aborting all still running tasks.
    NodeFailed {
        /// The outputs of the nodes already executed.
        outputs: Outputs,
        /// Index of the Node which returned an error. The error has also been logged.
        i: usize,
        /// The error returned by the node.
        error: E,
    },
}

impl<E: Er> std::fmt::Display for Error<E> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::NoNodes => write!(f, "No nodes in graph"),
            Error::NodeNotFound(type_id) => write!(f, "Node not found: {type_id:?}"),
            Error::DependencyNotFound(name, type_id) => {
                write!(f, "Node {name} has an unknown dependency: {type_id:?}")
            }
            Error::Cycle(names) => write!(f, "Found a cycle in your graph: {names:?}"),
            Error::Cancelled { .. } => write!(f, "Job was cancelled"),
            Error::NodePanic { error, .. } => write!(f, "Node panicked {error}"),
            Error::NodeFailed { i, error, .. } => {
                write!(f, "Node {i} failed with error: {error}")
            }
        }
    }
}
