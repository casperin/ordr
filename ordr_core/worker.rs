use std::{
    any::TypeId,
    collections::{HashMap, HashSet},
    sync::Arc,
    time::{Duration, Instant},
};

use serde_json::Value;
use tokio::{
    sync::Mutex,
    task::{JoinHandle, JoinSet},
};
use tracing::{error, info, warn};

use crate::{Context, Error, Job, Output, State};

enum Mode<S: State> {
    Init { job: Job<S>, state: S },
    Running(Instant, JoinHandle<Output>),
    Done(Output),
}

pub struct Worker<S: State> {
    out: Arc<Mutex<HashMap<&'static str, NodeState>>>,
    mode: Option<Mode<S>>, // Option because of ownership fun
}

impl<S: State> Worker<S> {
    pub fn new(job: Job<S>, state: S) -> Self {
        Self {
            out: Arc::new(Mutex::new(HashMap::new())),
            mode: Some(Mode::Init { job, state }),
        }
    }

    pub fn run(&mut self) -> Result<(), &'static str> {
        let Mode::Init { job, state } = std::mem::take(&mut self.mode).unwrap() else {
            return Err("Has already been started");
        };
        let t0 = Instant::now();
        let fut = run_job(job, state, self.out.clone(), t0);
        let handle = tokio::spawn(fut);
        self.mode = Some(Mode::Running(t0, handle));
        Ok(())
    }

    pub fn stop(&mut self) {
        let Mode::Running(t0, _) = self.mode.as_ref().unwrap() else {
            return;
        };
        let t = t0.elapsed();
        self.mode = Some(Mode::Done(Output::Stopped(t)));
    }

    pub async fn get_output(&mut self) -> Output {
        match self.mode.as_ref().unwrap() {
            // If we are done and have an output, then we just return that.
            Mode::Done(output) => return output.clone(),
            // If we haven't started, then we start and continue below.
            Mode::Init { .. } => self.run().unwrap(),
            // We need to take the handle, so we continue below.
            Mode::Running(_, _) => {}
        }
        // We know we are running, so we take the handle and wait for it.
        let Mode::Running(_, handle) = std::mem::take(&mut self.mode).unwrap() else {
            unreachable!();
        };

        let output = handle.await.expect("run_job should not be able to panic");
        self.mode = Some(Mode::Done(output.clone()));
        output
    }

    pub async fn data(&self) -> HashMap<&'static str, Value> {
        let mut data = HashMap::new();
        for (&name, state) in self.out.lock().await.iter() {
            if let NodeState::Provided(value) | NodeState::Done(_, _, value) = state {
                data.insert(name, value.clone());
            }
        }
        data
    }
}

#[derive(Debug)]
pub enum NodeState {
    Provided(Value),
    Running(Duration),
    Done(Duration, u32, Value),
    Retrying(u32, Duration),
    Failed(Duration, u32, Error),
}

pub async fn run_job<S: State>(
    job: Job<S>,
    state: S,
    out: Arc<Mutex<HashMap<&'static str, NodeState>>>,
    t0: Instant,
) -> Output {
    // Type for the JoinSet (or running tasks).
    enum Node {
        Done(TypeId, u32, Duration, Result<Value, Error>),
        Retry(TypeId, u32),
    }

    let nodes = job.nodes;
    let adj = job.adj;
    let mut results = HashMap::new();
    let mut handles = JoinSet::new();
    let mut abort_handles = HashMap::new();
    let mut pending: HashSet<TypeId> = nodes.keys().cloned().collect();

    let mut o = out.lock().await;
    for (id, (name, data)) in job.provided {
        info!(name, "Provided");
        o.insert(name, NodeState::Provided(data.clone()));
        results.insert(id, data);
    }
    drop(o);

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
            let state = NodeState::Running(t1);
            out.lock().await.insert(node.name, state);
            info!(node = node.name, "Node start");
            let abort_handle = handles.spawn(async move {
                let result = producer(context, payloads).await;
                Node::Done(id, 0, t0.elapsed(), result)
            });
            abort_handles.insert(abort_handle.id(), id);
        }

        let result = handles.join_next().await;
        let Some(result) = result else {
            let duration = t0.elapsed();
            info!(?duration, "Job done");
            return Output::Done(duration);
        };
        let result = match result {
            Ok(result) => result,
            Err(e) => {
                let t = t0.elapsed();
                let id = abort_handles[&e.id()];
                let name = nodes[&id].name;
                error!(name, "Node panicked");
                return Output::NodePanic(t, name, format!("{e:?}"));
            }
        };
        match result {
            Node::Done(id, retry, time, Ok(payload)) => {
                results.insert(id, payload.clone());
                let name = nodes[&id].name;
                let state = NodeState::Done(time, retry, payload);
                out.lock().await.insert(name, state);
                info!(name, "Node done");
            }
            Node::Done(id, retry, time, Err(e)) => match e.retry_in {
                Some(retry_in) => {
                    let name = nodes[&id].name;
                    warn!(name, retry, error = e.message, ?retry_in, "Node failed");
                    handles.spawn(async move {
                        tokio::time::sleep(time + retry_in).await;
                        Node::Retry(id, retry)
                    });
                }
                None => {
                    let t = t0.elapsed();
                    let name = nodes[&id].name;
                    let msg = format!("Node {name} failed ({retry} retries): {}", e.message);
                    let state = NodeState::Failed(time, retry, e);
                    out.lock().await.insert(name, state);
                    error!(name, "Node failed");
                    return Output::NodeFailed(t, name, msg);
                }
            },
            Node::Retry(id, mut retry) => {
                retry += 1;
                let payloads = get_payloads(id);
                let producer = nodes[&id].producer.clone();
                let t = t0.elapsed();
                let context = ctx(retry, t);
                let name = nodes[&id].name;
                let state = NodeState::Retrying(retry, t);
                out.lock().await.insert(name, state);
                info!(name, retry, "Node retrying");
                handles.spawn(async move {
                    let result = producer(context, payloads).await;
                    Node::Done(id, retry, t0.elapsed(), result)
                });
            }
        }
    }
}
