use std::{
    any::TypeId,
    collections::{HashMap, HashSet, hash_map::Entry},
};

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
        for name in self.data.keys() {
            warn!("Did not find {name} from the provided data. Discarding.");
        }
        if let Some(cycle) = find_cycle(&self.job.adj) {
            let names: Vec<_> = cycle.iter().map(|id| self.job.nodes[id].name).collect();
            return Err(format!("Cycle found: {}", names.join(" -> ")));
        }
        let mut seen = HashSet::new();
        for node in self.job.nodes.values() {
            if seen.contains(node.name) {
                return Err(format!("Found two nodes with the same name: {}", node.name));
            }
            seen.insert(node.name);
        }
        Ok(self.job)
    }
}

pub fn find_cycle(adj: &HashMap<TypeId, Vec<TypeId>>) -> Option<Vec<TypeId>> {
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
        let a = TypeId::of::<u16>();
        let b = TypeId::of::<u32>();
        let c = TypeId::of::<u64>();
        let d = TypeId::of::<i64>();
        let e = TypeId::of::<i64>();
        let f = TypeId::of::<i64>();
        let mut adj = HashMap::new();
        adj.insert(a, vec![b, c, f]);
        adj.insert(b, vec![d]);
        adj.insert(c, vec![e]);
        adj.insert(d, vec![e]);
        adj.insert(e, vec![b]);
        let result = find_cycle(&adj);
        assert!(result.is_some());
    }
}
