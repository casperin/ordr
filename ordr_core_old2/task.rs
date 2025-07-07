use std::{
    any::TypeId,
    collections::{HashMap, HashSet},
    time::Duration,
};

use serde::Serialize;
use tracing::warn;

use crate::{
    graph::Graph,
    node::{Context, Error, Node, State},
};

pub(crate) enum NodeState {
    Inactive,
    Provided(Vec<u8>),
    Target,
    Pending,
    Running(Duration),
    Done(Duration, Duration, Vec<u8>),
    Failed(Duration, Duration, Duration, Error),
    Fatal(Duration, Duration, Error),
    Aborted(Duration, Duration),
}

pub struct Builder<'a, S: State> {
    graph: &'a Graph<S>,
    targets: Vec<TypeId>,
    provided: HashMap<TypeId, Vec<u8>>,
}

impl<'a, S: State> Builder<'a, S> {
    pub(crate) fn new(graph: &'a Graph<S>) -> Self {
        Self {
            graph,
            targets: vec![],
            provided: HashMap::new(),
        }
    }

    fn get_id(&self, name: &str) -> Result<TypeId, String> {
        if let Some(id) = self.graph.ids.get(name) {
            return Ok(*id);
        };
        let available = self
            .graph
            .ids
            .keys()
            .cloned()
            .collect::<Vec<_>>()
            .join(", ");
        let e = format!("Not found: {name}, available: {available}");
        Err(e)
    }

    pub fn add_target(&mut self, target_name: &str) -> Result<(), String> {
        let id = self.get_id(target_name)?;
        self.targets.push(id);
        Ok(())
    }

    pub fn target(mut self, target_name: &str) -> Result<Self, String> {
        self.add_target(target_name)?;
        Ok(self)
    }

    pub fn targets(mut self, target_names: &[&str]) -> Result<Self, String> {
        for target_name in target_names {
            self.add_target(target_name)?;
        }
        Ok(self)
    }

    pub fn add_data<T: Node<S> + Serialize>(&mut self, payload: T) -> Result<(), String> {
        let id = T::id();

        // Ensure we aren't adding data for no reason, leaving the user
        // wondering why we are ignoring it.
        let id_found = self.graph.names.contains_key(&id);
        if !id_found {
            let e = "No node in the graph corresponds to the data you added.";
            return Err(e.into());
        }

        match serde_cbor::to_vec(&payload) {
            Ok(payload) => {
                self.provided.insert(T::id(), payload);
                Ok(())
            }
            Err(e) => {
                let e = format!("Failed to serialize your payload (we use cbor): {e:#}");
                Err(e)
            }
        }
    }

    pub fn build(self) -> Task {
        let mut stack: Vec<TypeId> = vec![];
        let mut inner: HashMap<TypeId, NodeState> = self
            .graph
            .names
            .keys()
            .map(|id| (*id, NodeState::Inactive))
            .collect();

        for (id, payload) in self.provided {
            let state = NodeState::Provided(payload);
            inner.insert(id, state);
        }

        for id in self.targets {
            if matches!(inner[&id], NodeState::Inactive) {
                inner.insert(id, NodeState::Target);
                stack.extend(&self.graph.deps[&id]);
            }
        }

        while let Some(id) = stack.pop() {
            if matches!(inner[&id], NodeState::Inactive) {
                inner.insert(id, NodeState::Pending);
                stack.extend(&self.graph.deps[&id]);
            }
        }

        Task { inner }
    }
}

pub struct Task {
    inner: HashMap<TypeId, NodeState>,
}

impl Task {
    pub(crate) fn pending(&self) -> HashSet<TypeId> {
        self.inner
            .iter()
            .filter(|(_id, ns)| matches!(*ns, NodeState::Pending | NodeState::Target))
            .map(|(id, _)| *id)
            .collect()
    }

    /// Get all payloads, or get none.
    pub(crate) fn get_payloads(&self, deps: &[TypeId]) -> Option<Vec<Vec<u8>>> {
        let mut payloads = vec![];

        // First borrow the payloads, in case (bar far the most common) we
        // don't have a payload.
        for dep in deps {
            let payload = self.get_payload(dep)?;
            payloads.push(payload);
        }

        // We have all payloads, so we clone them.

        Some(payloads.into_iter().map(|p| p.to_vec()).collect())
    }

    fn get_payload(&self, id: &TypeId) -> Option<&[u8]> {
        match &self.inner[id] {
            NodeState::Provided(payload) | NodeState::Done(_, _, payload) => Some(payload),
            _ => None,
        }
    }

    // pub fn next<S: State>(
    //     &mut self,
    //     now: Duration,
    //     deps: &HashMap<TypeId, Vec<TypeId>>,
    //     state: &S,
    // ) -> Action<S> {
    //     let mut ready_ids = vec![];
    //     for (id, node_state) in &self.inner {
    //         if matches!(node_state, NodeState::Pending) {
    //             if let Some(payloads) = self.get_payloads(&deps[id]) {
    //                 ready_ids.push((*id, 0, payloads));
    //             }
    //         }
    //     }
    //     // We have ready tasks.
    //     if !ready_ids.is_empty() {
    //         let mut ready = vec![];
    //         for (id, retry, payloads) in ready_ids {
    //             let context = Context {
    //                 state: state.clone(),
    //                 retry,
    //             };
    //             ready.push((id, context, payloads));
    //             self.inner.insert(id, NodeState::Running(now));
    //         }
    //         return Action::Start(ready);
    //     }
    //     Action::Nothing
    // }

    // pub fn node_done(&mut self, id: TypeId, stop: Duration, result: Result<Vec<u8>, Error>) {
    //     let NodeState::Running(start) = self.inner[&id] else {
    //         warn!("{id:?} claims to be done, but it wasn't running");
    //         return;
    //     };
    //     match result {
    //         Ok(payload) => {
    //             let ns = NodeState::Done(start, stop, payload);
    //             self.inner.insert(id, ns);
    //         }
    //         Err(e) => match e.retry_in {
    //             Some(retry) => {
    //                 let ns = NodeState::Failed(start, stop, stop + retry, e);
    //                 self.inner.insert(id, ns);
    //             }
    //             None => {
    //                 let ns = NodeState::Fatal(start, stop, e);
    //                 self.inner.insert(id, ns);
    //             }
    //         },
    //     }
    // }
}

pub enum Action<S: State> {
    Nothing,
    Start(Vec<(TypeId, Context<S>, Vec<Vec<u8>>)>),
}
