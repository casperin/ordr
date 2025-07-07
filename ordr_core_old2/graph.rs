use std::{any::TypeId, collections::HashMap};

use crate::{
    node::{Node, Producer, State},
    task,
};

#[derive(Clone)]
pub struct Graph<S: State> {
    pub(crate) ids: HashMap<&'static str, TypeId>,
    pub(crate) names: HashMap<TypeId, &'static str>,
    pub(crate) deps: HashMap<TypeId, Vec<TypeId>>,
    pub(crate) producers: HashMap<TypeId, Producer<S>>,
}

impl<S: State> Graph<S> {
    pub fn builder() -> Builder<S> {
        Builder {
            graph: Graph {
                ids: HashMap::new(),
                names: HashMap::new(),
                deps: HashMap::new(),
                producers: HashMap::new(),
            },
        }
    }

    pub fn task_builder(&self) -> task::Builder<S> {
        task::Builder::new(self)
    }
}

impl<S: State> std::fmt::Debug for Graph<S> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Graph<{}>", self.names.len())
    }
}

pub struct Builder<S: State> {
    graph: Graph<S>,
}

impl<S: State> Builder<S> {
    pub fn add_node<N: Node<S>>(&mut self) {
        self.graph.ids.insert(N::name(), N::id());
        self.graph.names.insert(N::id(), N::name());
        self.graph.deps.insert(N::id(), N::deps());
        self.graph.producers.insert(N::id(), N::producer());
    }

    pub fn node<N: Node<S>>(mut self) -> Self {
        self.add_node::<N>();
        self
    }

    pub fn build(self) -> Result<Graph<S>, String> {
        validate(&self.graph)?;
        Ok(self.graph)
    }
}

fn validate<S: State>(graph: &Graph<S>) -> Result<(), String> {
    for (id, deps) in &graph.deps {
        let all_there = deps.iter().all(|dep| graph.names.contains_key(dep));
        if !all_there {
            let e = format!(
                "Graph does not contain all dependencies for {}",
                graph.names[id]
            );
            return Err(e);
        }
    }
    // TODO: Validate no cycles
    Ok(())
}
