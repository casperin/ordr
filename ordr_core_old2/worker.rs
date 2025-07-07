use std::{
    any::TypeId,
    collections::HashMap,
    time::{Duration, Instant},
};

use tokio::{
    select,
    sync::mpsc::Sender,
    task::{JoinError, JoinSet},
};

use crate::{
    graph::Graph,
    node::{Context, Error, State},
    task::Task,
};

pub struct Worker<S: State> {
    state: WorkerState<S>,
}

enum WorkerState<S: State> {
    Init(Graph<S>, Task),
}

impl<S: State> Worker<S> {
    pub fn new(graph: Graph<S>, task: Task) -> Self {
        Self {
            state: WorkerState::Init(graph, task),
        }
    }

    pub async fn run(&mut self, state: S) {
        // ...
    }
}

enum Msg {
    NodeStart(TypeId, Duration),
    NodeDone(TypeId, Duration, Vec<u8>),
    NodeRetrying(TypeId, u32, Duration),
    NodeFailed(TypeId, Duration, Error),
    NodePanicked(JoinError, Duration),
    Done(Duration),
}

async fn run<S: State>(graph: Graph<S>, task: Task, state: S, tx: Sender<Msg>) {
    // Type for the JoinSet (or running tasks).
    enum Job {
        Done(TypeId, u32, Duration, Result<Vec<u8>, Error>),
        Retry(TypeId, u32),
    }

    // Start time for running this job. We only use this to calculate durations.
    let t0 = Instant::now();
    // All our running nodes.
    let mut handles = JoinSet::new();
    // Find all the tasks we need to run.
    let mut pending = task.pending();
    // We collect all our good results here, so we can look them up quickly.
    let mut results: HashMap<TypeId, Vec<u8>> = HashMap::new();

    // A helper to create a Context.
    let ctx = |retry, start| Context {
        retry,
        start,
        state: state.clone(),
    };

    loop {
        // A few helper functions.
        let is_done = |i| results.contains_key(i);
        let get_payloads = |id| {
            graph.deps[&id]
                .iter()
                .map(|id| results[&id].clone())
                .collect()
        };

        // Start the ready nodes.
        let ready = pending.extract_if(|id| graph.deps[id].iter().all(is_done));
        for id in ready {
            let payloads = get_payloads(id);
            let producer = graph.producers[&id].clone();
            let t1 = t0.elapsed();
            let context = ctx(0, t1);
            tx.send(Msg::NodeStart(id, t1)).await;
            handles.spawn(async move {
                let result = producer(context, payloads).await;
                Job::Done(id, 0, t0.elapsed(), result)
            });
        }

        let result = handles.join_next().await;
        let Some(result) = result else {
            tx.send(Msg::Done(t0.elapsed())).await;
            return;
        };
        let result = match result {
            Ok(result) => result,
            Err(e) => {
                tx.send(Msg::NodePanicked(e, t0.elapsed())).await;
                return;
            }
        };
        match result {
            Job::Done(id, _retry, time, Ok(payload)) => {
                results.insert(id, payload.clone());
                tx.send(Msg::NodeDone(id, time, payload)).await;
            }
            Job::Done(id, retry, time, Err(e)) => match e.retry_in {
                Some(retry_in) => {
                    handles.spawn(async move {
                        tokio::time::sleep(time + retry_in).await;
                        Job::Retry(id, retry + 1)
                    });
                }
                None => {
                    tx.send(Msg::NodeFailed(id, time, e)).await;
                    return;
                }
            },
            Job::Retry(id, retry) => {
                let payloads = get_payloads(id);
                let producer = graph.producers[&id].clone();
                let t = t0.elapsed();
                let context = ctx(retry, t);
                tx.send(Msg::NodeRetrying(id, retry, t)).await;
                handles.spawn(async move {
                    let result = producer(context, payloads).await;
                    Job::Done(id, retry, t0.elapsed(), result)
                });
            }
        }
    }
}
