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

use crate::{Context, Error, Job, State};

enum Mode<S: State> {
    Init {
        job: Job<S>,
        state: S,
    },
    Starting,
    Running {
        handle: JoinHandle<Result<(), String>>,
    },
    Done,
    Stopped,
}

pub struct Worker<S: State> {
    out: Arc<Mutex<HashMap<&'static str, NodeState>>>,
    mode: Mode<S>,
}

impl<S: State> Worker<S> {
    pub fn new(job: Job<S>, state: S) -> Self {
        Self {
            out: Arc::new(Mutex::new(HashMap::new())),
            mode: Mode::Init { job, state },
        }
    }

    pub fn run(&mut self) -> Result<(), &'static str> {
        let Mode::Init { job, state } = std::mem::replace(&mut self.mode, Mode::Starting) else {
            return Err("Has already been started");
        };
        let fut = run_job(job, state, self.out.clone());
        let handle = tokio::spawn(fut);
        self.mode = Mode::Running { handle };
        Ok(())
    }

    pub fn stop(&mut self) {
        println!("1");
        if matches!(&self.mode, Mode::Running { .. }) {
            println!("2");
            self.mode = Mode::Stopped;
        }
    }

    pub async fn wait_for_job(&mut self) -> Result<(), String> {
        if matches!(&self.mode, Mode::Init { .. }) {
            self.run().unwrap();
        }
        if !matches!(self.mode, Mode::Running { .. }) {
            return Ok(());
        }
        let Mode::Running { handle } = std::mem::replace(&mut self.mode, Mode::Done) else {
            unreachable!();
        };
        match handle.into_future().await {
            Ok(result) => result,
            Err(join_error) => Err(format!("Job panicked: {join_error:?}")),
        }
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
) -> Result<(), String> {
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
            handles.spawn(async move {
                let result = producer(context, payloads).await;
                Job::Done(id, 0, t0.elapsed(), result)
            });
        }

        let result = handles.join_next().await;
        let Some(result) = result else {
            info!("Job done");
            return Ok(());
        };
        let result = match result {
            Ok(result) => result,
            Err(e) => {
                error!("Node panicked");
                return Err(format!("Node panicked: {e:?}"));
            }
        };
        match result {
            Job::Done(id, retry, time, Ok(payload)) => {
                results.insert(id, payload.clone());
                let name = nodes[&id].name;
                let state = NodeState::Done(time, retry, payload);
                out.lock().await.insert(name, state);
                info!(name, "Node done");
            }
            Job::Done(id, retry, time, Err(e)) => match e.retry_in {
                Some(retry_in) => {
                    let name = nodes[&id].name;
                    warn!(name, retry, ?retry_in, "Node failed");
                    handles.spawn(async move {
                        tokio::time::sleep(time + retry_in).await;
                        Job::Retry(id, retry)
                    });
                }
                None => {
                    let name = nodes[&id].name;
                    let msg = format!("Node {name} failed ({retry} retries): {}", e.message);
                    let state = NodeState::Failed(time, retry, e);
                    out.lock().await.insert(name, state);
                    error!(name, "Node failed");
                    return Err(msg);
                }
            },
            Job::Retry(id, mut retry) => {
                retry += 1;
                let payloads = get_payloads(id);
                let producer = nodes[&id].producer.clone();
                let t = t0.elapsed();
                let context = ctx(retry, t);
                let name = nodes[&id].name;
                let state = NodeState::Retrying(retry, t);
                out.lock().await.insert(name, state);
                info!(retry, "Node retrying");
                handles.spawn(async move {
                    let result = producer(context, payloads).await;
                    Job::Done(id, retry, t0.elapsed(), result)
                });
            }
        }
    }
}
