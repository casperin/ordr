#![allow(unused)]

mod node_state;

use std::{
    sync::Arc,
    thread::JoinHandle,
    time::{Duration, Instant},
};

use node_state::NodeStates;
use tokio::{sync::Mutex, task::JoinSet};

use crate::{
    graph::{Ctx, Er, Graph},
    job::Job,
    node::Payload,
};

enum State {
    Init,
    Running(Instant, JoinHandle<()>),
}

pub struct Worker<C: Ctx, E: Er> {
    graph: Graph<C, E>,
    job: Job<C, E>,
    state: State,
}

impl<C: Ctx, E: Er> Worker<C, E> {}

async fn run<C: Ctx, E: Er>(graph: &Graph<C, E>, node_states: Arc<Mutex<NodeStates<E>>>) {
    let mut handles = JoinSet::new();
    let mut pending = node_states.lock().await.pending();

    loop {
        {
            let states = node_states.lock().await;
            let ready = pending.extract_if(.., |i| states.is_ready(&graph.adj, i));
        }
        // ...
    }
}
