#![warn(missing_docs)]

//! The core of Ordr. Almost everything is implemented here.

/// Error returned ordr's various functions that may fail.
pub mod error;

/// Collection of Nodes that can execute Jobs.
pub mod graph;

/// Create a for the graph to execute.
pub mod job;

/// For creating mermaid diagrams.
mod mermaid;

/// Nodes in the graph. This is exposed so the macros can reach them. Users should probably not need to look at these.
pub mod node;

/// A collection of outputs produced by the various nodes in a graph.
pub mod outputs;

/// Validation of graphs and jobs.
mod validation;
