use ::std::hash::BuildHasher;
use std::{
    any::TypeId,
    collections::{HashMap, HashSet, hash_map::Entry},
    fmt,
};

use serde_json::Value;
use tracing::warn;

use crate::{Node, NodeBuilder, State};

/// Describes what needs to be done, and how to do it. Pass it to a [`crate::Worker`] to have it
/// executed.
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
    #[must_use]
    pub fn builder() -> JobBuilder<S> {
        JobBuilder {
            data: HashMap::new(),
            job: Job::default(),
        }
    }

    #[must_use]
    pub fn builder_with_data(data: HashMap<String, Value>) -> JobBuilder<S> {
        JobBuilder {
            data,
            job: Job::default(),
        }
    }

    /// Returns the number of nodes in this [`Job<S>`].
    #[must_use]
    pub fn len(&self) -> usize {
        self.nodes.len()
    }

    /// Returns `true` if this [`Job<S>`] contains no nodes.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }

    #[must_use]
    pub fn name(&self, id: &TypeId) -> &'static str {
        self.nodes[id].name
    }
}

/// Builds a job. Created with [`Job::builder()`]. Call `.build()` on it to create a [`Job`].
pub struct JobBuilder<S: State> {
    data: HashMap<String, Value>,
    job: Job<S>,
}

impl<S: State> JobBuilder<S> {
    /// Adds a node to the job. All dependencies of the node will be automatically added as well.
    #[must_use]
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
    ///
    /// # Errors
    /// If the graph contains any cycles, or if there is a name collision.
    pub fn build(self) -> Result<Job<S>, JobError> {
        for name in self.data.keys() {
            warn!("Did not find {name} from the provided data. Discarding.");
        }
        if let Some(cycle) = find_cycle(&self.job.adj) {
            let names: Vec<_> = cycle.iter().map(|id| self.job.nodes[id].name).collect();
            return Err(JobError::Cycle(names));
        }
        let mut seen = HashSet::new();
        for node in self.job.nodes.values() {
            if seen.contains(node.name) {
                return Err(JobError::DuplicateName(node.name));
            }
            seen.insert(node.name);
        }
        Ok(self.job)
    }
}

#[derive(Debug)]
pub enum JobError {
    Cycle(Vec<&'static str>),
    DuplicateName(&'static str),
}

impl fmt::Display for JobError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            JobError::Cycle(names) => write!(f, "Cycle found: {}", names.join(" -> ")),
            JobError::DuplicateName(name) => {
                write!(f, "Found two nodes with the same name: {name}")
            }
        }
    }
}

impl std::error::Error for JobError {}

#[must_use]
fn find_cycle<S: BuildHasher>(adj: &HashMap<TypeId, Vec<TypeId>, S>) -> Option<Vec<TypeId>> {
    // Keep track of the nodes: None = not seen, Some(false) = visiting, Some(true) = done.
    let mut state = HashMap::new();
    // Used to build the path if we find a cycle.
    let mut parents = HashMap::new();
    let mut stack = vec![];

    // Loop through all nodes as a potential starting point.
    for id in adj.keys() {
        // If we have already checked it out, then disregard it.
        if state.contains_key(id) {
            continue;
        }

        // Push our starting point onto the stack.
        stack.push(*id);

        while let Some(&id) = stack.last() {
            match state.entry(id) {
                Entry::Occupied(mut e) => {
                    e.insert(true); // Done
                    stack.pop();
                }
                Entry::Vacant(e) => {
                    e.insert(false); // Visiting
                }
            }

            let Some(deps) = adj.get(&id) else {
                continue;
            };

            for &dep in deps {
                parents.insert(dep, id);

                match state.get(&dep) {
                    None => stack.push(dep),
                    Some(true) => {} // Already done, noop
                    Some(false) => {
                        // Cycle found! Build the path and return it
                        let mut parent = parents[&dep];
                        let mut path = vec![dep, parent];
                        while parent != dep {
                            parent = parents[&parent];
                            path.push(parent);
                        }
                        return Some(path);
                    }
                }
            }
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use std::{any::TypeId, collections::HashMap};

    use super::find_cycle;

    #[test]
    fn can_find_simple_cycle() {
        let a = TypeId::of::<u16>();
        let b = TypeId::of::<u32>();
        let c = TypeId::of::<u64>();
        let mut adj = HashMap::new();
        adj.insert(a, vec![b]);
        adj.insert(b, vec![c]);
        adj.insert(c, vec![a]);
        let result = find_cycle(&adj);
        assert!(result.is_some());
    }

    #[test]
    fn can_not_find_a_not_cycle() {
        let a = TypeId::of::<u16>();
        let b = TypeId::of::<u32>();
        let c = TypeId::of::<u64>();
        let mut adj = HashMap::new();
        adj.insert(a, vec![c]);
        adj.insert(b, vec![c]);
        adj.insert(c, vec![]);
        let result = find_cycle(&adj);
        assert!(result.is_none());
    }

    #[test]
    fn can_find_cycle() {
        let node_a = TypeId::of::<u16>();
        let node_b = TypeId::of::<u32>();
        let node_c = TypeId::of::<u64>();
        let node_d = TypeId::of::<i64>();
        let node_e = TypeId::of::<i64>();
        let node_f = TypeId::of::<i64>();
        let mut adj = HashMap::new();
        adj.insert(node_a, vec![node_b, node_c, node_f]);
        adj.insert(node_b, vec![node_d]);
        adj.insert(node_c, vec![node_e]);
        adj.insert(node_d, vec![node_e]);
        adj.insert(node_e, vec![node_b]);
        let result = find_cycle(&adj);
        assert!(result.is_some());
    }
}
