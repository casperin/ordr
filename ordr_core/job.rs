use std::{
    any::TypeId,
    collections::{HashMap, HashSet, hash_map::Entry},
};

use crate::{Node, NodeBuilder, State};

#[macro_export]
macro_rules! new_job {
    ($($ty:ty),* $(,)?) => {{
        let mut b = $crate::Job::builder();
        $(
            b.add::<$ty>();
        )*
        b.build()
    }};
}

#[derive(Debug, Clone)]
pub struct Job<S: State> {
    pub(crate) targets: Vec<TypeId>,
    pub(crate) nodes: HashMap<TypeId, Node<S>>,
    pub(crate) adj: HashMap<TypeId, Vec<TypeId>>,
}

impl<S: State> Job<S> {
    pub fn builder() -> JobBuilder<S> {
        JobBuilder {
            targets: HashSet::new(),
            nodes: HashMap::new(),
            adj: HashMap::new(),
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
    targets: HashSet<TypeId>,
    nodes: HashMap<TypeId, Node<S>>,
    adj: HashMap<TypeId, Vec<TypeId>>,
}

impl<S: State> JobBuilder<S> {
    /// Adds a node to the job. All dependencies of the node will be automatically added as well.
    pub fn add<N: NodeBuilder<S>>(&mut self) {
        let node = N::node();
        self.targets.insert(node.id);
        // Use a stack to recursively add dependencies.
        let mut stack = vec![node];
        while let Some(node) = stack.pop() {
            // Only add node if we don't already have it.
            if let Entry::Vacant(entry) = self.nodes.entry(node.id) {
                let deps = (node.deps)();
                let dep_ids = deps.iter().map(|n| n.id).collect();
                self.adj.insert(node.id, dep_ids);
                stack.extend(deps);
                entry.insert(node);
            }
        }
    }

    /// Creates and validates the Job.
    /// # Errors
    /// If there are any cycles.
    pub fn build(self) -> Result<Job<S>, String> {
        // TODO: Validate absence of cycles.
        // Maybe we should also ensure that names are unique?
        Ok(Job {
            targets: self.targets.into_iter().collect(),
            nodes: self.nodes,
            adj: self.adj,
        })
    }
}
