use std::{sync::Arc, time::Duration};

use tokio::sync::Mutex;

use crate::{
    error::Error,
    graph::{Ctx, Er, Graph},
    job::Job,
    node::Payload,
};

pub(crate) struct NodeStates<E: Er> {
    states: Vec<NodeState<E>>,
}

impl<E: Er> NodeStates<E> {
    pub fn node_states<C: Ctx>(graph: &Graph<C, E>, job: Job<C, E>) -> Self {
        let len = graph.nodes.len();
        // Create initial state where everything is inactive. Then walk though
        // the adj and update them.
        let mut states: Vec<NodeState<E>> = (0..len).map(|_| NodeState::Inactive).collect();
        let mut stack: Vec<usize> = vec![]; // for graph traversal

        // Set `Provided`
        for (id, payload) in job.inputs {
            if let Ok(i) = graph.nodes.binary_search_by_key(&id, |n| n.id) {
                states[i] = NodeState::Provided { payload };
            }
        }

        // Set `Targets`
        for id in job.targets {
            let Ok(i) = graph.nodes.binary_search_by_key(&id, |n| n.id) else {
                continue;
            };
            // If it's already provided, then we don't set it as a target.
            if matches!(states[i], NodeState::Inactive) {
                states[i] = NodeState::Target;
                stack.extend(&graph.adj[i]); // Add its deps for graph traversal
            }
        }

        // Set `Pending`
        while let Some(i) = stack.pop() {
            // We only update inactive state because
            // * Provided No need to run those
            // * Target   Sort of already pending
            // * Pending  Already set (incl. deps)
            if matches!(states[i], NodeState::Inactive) {
                states[i] = NodeState::Pending;
                stack.extend(&graph.adj[i]);
            }
        }

        Self { states }
    }

    pub fn pending(&self) -> Vec<usize> {
        self.states
            .iter()
            .enumerate()
            .filter(|(_, n)| matches!(n, NodeState::Pending))
            .map(|(i, _)| i)
            .collect()
    }
}

pub(crate) enum NodeState<E: Er> {
    /// Not used for running this particular job.
    Inactive,
    /// Provided by the user.
    Provided {
        payload: Payload,
    },
    Target,
    Pending,
    Running {
        start: Duration,
    },
    /// Finished successfully.
    Done {
        start: Duration,
        stop: Duration,
        payload: Payload,
    },
    Failed {
        start: Duration,
        stop: Duration,
        error: E,
    },
    Aborted {
        start: Duration,
        stop: Duration,
    },
}

impl<E: Er> NodeState<E> {
    fn payload(&self) -> Option<&Payload> {
        match self {
            NodeState::Provided { payload } | NodeState::Done { payload, .. } => Some(&payload),
            _ => None,
        }
    }
}
