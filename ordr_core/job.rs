use std::{
    any::TypeId,
    collections::{HashMap, HashSet, hash_map::Entry},
};

use serde::Serialize;
use serde_json::Value;
use tracing::warn;

use crate::{Node, NodeBuilder, State};

#[derive(Debug, Clone)]
pub struct Job<S: State> {
    pub(crate) nodes: HashMap<TypeId, Node<S>>,
    pub(crate) adj: HashMap<TypeId, Vec<TypeId>>,
    pub(crate) provided: HashMap<TypeId, (&'static str, Value)>,
}

impl<S: State> Default for Job<S> {
    fn default() -> Self {
        Job {
            nodes: HashMap::new(),
            adj: HashMap::new(),
            provided: HashMap::new(),
        }
    }
}

impl<S: State> Job<S> {
    pub fn builder() -> JobBuilder<S> {
        JobBuilder {
            data: HashMap::new(),
            job: Job::default(),
        }
    }

    pub fn builder_with_data(data: HashMap<String, Value>) -> JobBuilder<S> {
        JobBuilder {
            data,
            job: Job::default(),
        }
    }

    /// Returns the number of nodes in this [`Job<S>`].
    pub fn len(&self) -> usize {
        self.nodes.len()
    }

    /// Returns `true` if this [`Job<S>`] contains no nodes.
    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }

    pub fn name(&self, id: &TypeId) -> &'static str {
        self.nodes[id].name
    }
}

pub struct JobBuilder<S: State> {
    data: HashMap<String, Value>,
    job: Job<S>,
}

impl<S: State> JobBuilder<S> {
    /// Adds a node to the job. All dependencies of the node will be automatically added as well.
    pub fn add<N: NodeBuilder<S>>(mut self) -> Self {
        // Use a stack to recursively add dependencies.
        let mut stack = vec![N::node()];
        while let Some(node) = stack.pop() {
            // If we already have it `data`, then we promote the data item to actual provided data
            // under its id.
            if let Some(data) = self.data.remove(node.name) {
                self.job.provided.insert(node.id, (node.name, data));
                continue;
            }
            // If it was already promoted, then we should just ignore it.
            if self.job.provided.contains_key(&node.id) {
                continue;
            }
            // Only add node if we don't already have it.
            if let Entry::Vacant(entry) = self.job.nodes.entry(node.id) {
                let deps = (node.deps)();
                let dep_ids = deps.iter().map(|n| n.id).collect();
                self.job.adj.insert(node.id, dep_ids);
                stack.extend(deps);
                entry.insert(node);
            }
        }
        self
    }

    /// Creates and validates the Job.
    pub fn build(self) -> Result<Job<S>, String> {
        // TODO: Validate:
        // * Cycles
        // * Names are unique
        // * Probably should be nothing left in self.data
        for name in self.data.keys() {
            warn!("Did not find {name} from the provided data. Discarding.");
        }
        Ok(self.job)
    }
}
