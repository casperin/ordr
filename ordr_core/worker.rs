use std::{
    any::TypeId,
    collections::{HashMap, HashSet},
    time::{Duration, Instant},
};

use serde_json::Value;
use tokio::{
    sync::{OwnedMappedMutexGuard, mpsc::Sender},
    task::{JoinError, JoinSet},
};
use tracing::{error, info};

use crate::{Context, Error, Job, State};

pub struct Worker;

impl Worker {
    pub async fn run<S: State>(
        job: Job<S>,
        state: S,
    ) -> (
        HashMap<&'static str, Value>,
        Result<(), (&'static str, String)>,
    ) {
        let (tx, mut rx) = tokio::sync::mpsc::channel(job.len());
        let p = run_job(job, state, tx);
        tokio::spawn(p);

        let mut results = HashMap::new();
        let mut out = Ok(());

        while let Some(msg) = rx.recv().await {
            // println!("{msg:?}");
            match msg {
                Msg::Provided(name, v) => {
                    results.insert(name, v);
                }
                Msg::NodeDone(name, t, v) => {
                    info!(name, ?t, "Node done");
                    results.insert(name, v);
                }
                Msg::NodeStart(name, t) => {
                    info!(name, ?t, "Node start");
                }
                Msg::NodeFailed(name, t, e) => {
                    let e = e.message;
                    error!(name, e, ?t, "Node failed");
                    out = Err((name, e));
                }
                Msg::NodeRetrying(name, retry, t) => {
                    info!(name, retry, ?t, "Node retrying");
                }
                Msg::NodePanicked(join_error, t) => {
                    error!(?join_error, ?t, "Node panicked");
                }
                Msg::Done(t) => {
                    info!(?t, "Job done");
                }
            };
        }

        (results, out)
    }
}

#[derive(Debug)]
pub enum Msg {
    Provided(&'static str, Value),
    NodeStart(&'static str, Duration),
    NodeDone(&'static str, Duration, Value),
    NodeRetrying(&'static str, u32, Duration),
    NodeFailed(&'static str, Duration, Error),
    NodePanicked(JoinError, Duration),
    Done(Duration),
}

pub async fn run_job<S: State>(job: Job<S>, state: S, tx: Sender<Msg>) {
    // Type for the JoinSet (or running tasks).
    enum Job {
        Done(TypeId, u32, Duration, Result<Value, Error>),
        Retry(TypeId, u32),
    }

    let nodes = job.nodes;
    let adj = job.adj;
    let mut results = HashMap::new();
    let t0 = Instant::now();
    let mut handles = JoinSet::new();
    let mut pending: HashSet<TypeId> = nodes.keys().cloned().collect();

    for (id, (name, data)) in job.provided {
        tx.send(Msg::Provided(name, data.clone())).await;
        results.insert(id, data);
    }

    // A helper to create a Context.
    let ctx = |retry, start| Context {
        retry,
        start,
        state: state.clone(),
    };

    loop {
        // A few helper functions.
        let is_done = |i| results.contains_key(i);
        let get_payloads = |id| adj[&id].iter().map(|id| results[id].clone()).collect();

        // Start the ready nodes.
        let ready = pending.extract_if(|id| adj[id].iter().all(is_done));
        for id in ready {
            let payloads = get_payloads(id);
            let node = &nodes[&id];
            let producer = node.producer.clone();
            let t1 = t0.elapsed();
            let context = ctx(0, t1);
            tx.send(Msg::NodeStart(node.name, t1)).await;
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
                tx.send(Msg::NodeDone(nodes[&id].name, time, payload)).await;
            }
            Job::Done(id, retry, time, Err(e)) => match e.retry_in {
                Some(retry_in) => {
                    handles.spawn(async move {
                        tokio::time::sleep(time + retry_in).await;
                        Job::Retry(id, retry + 1)
                    });
                }
                None => {
                    tx.send(Msg::NodeFailed(nodes[&id].name, time, e)).await;
                    return;
                }
            },
            Job::Retry(id, retry) => {
                let payloads = get_payloads(id);
                let producer = nodes[&id].producer.clone();
                let t = t0.elapsed();
                let context = ctx(retry, t);
                tx.send(Msg::NodeRetrying(nodes[&id].name, retry, t)).await;
                handles.spawn(async move {
                    let result = producer(context, payloads).await;
                    Job::Done(id, retry, t0.elapsed(), result)
                });
            }
        }
    }
}
