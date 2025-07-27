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

/// Runs [`crate::Job`]s.
#[derive(Clone)]
pub struct Worker<S: State> {
    out: Arc<Mutex<HashMap<&'static str, NodeState>>>,
    mode: Arc<Mutex<Option<Mode<S>>>>, // Option because of ownership fun
}

impl<S: State> Worker<S> {
    /// Create a new worker.
    pub fn new(job: Job<S>, state: S) -> Self {
        Self {
            out: Arc::new(Mutex::new(HashMap::new())),
            mode: Arc::new(Mutex::new(Some(Mode::Init { job, state }))),
        }
    }

    /// Start running the job.
    ///
    /// # Errors
    /// If the worker has already started working.
    #[allow(clippy::missing_panics_doc)]
    pub async fn run(&mut self) -> Result<(), &'static str> {
        let mut mode = self.mode.lock().await;
        let Mode::Init { job, state } = std::mem::take(&mut *mode).unwrap() else {
            return Err("Has already been started");
        };
        let t0 = Instant::now();
        let fut = run_job(job, state, self.out.clone(), t0);
        let handle = tokio::spawn(fut);
        *mode = Some(Mode::Running(t0, handle));
        Ok(())
    }

    /// Stop the worker. All currently running nodes will be aborted.
    #[allow(clippy::missing_panics_doc)]
    pub async fn stop(&mut self) {
        let mut mode = self.mode.lock().await;
        let Mode::Running(t0, _) = mode.as_ref().unwrap() else {
            return;
        };
        let duration = t0.elapsed();
        *mode = Some(Mode::Done(Output::Stopped { duration }));
    }

    /// Wait for the worker to finish and return the [`Output`].
    ///
    /// # Errors
    /// If the worker is not yet running.
    #[allow(clippy::missing_panics_doc)]
    pub async fn get_output(&mut self) -> Result<Output, &'static str> {
        let mut mode = self.mode.lock().await;
        match mode.as_ref().unwrap() {
            // If we are done and have an output, then we just return that.
            Mode::Done(output) => return Ok(output.clone()),
            // If we haven't started, then we start and continue below.
            Mode::Init { .. } => return Err("Not running"),
            // We need to take the handle, so we continue below.
            Mode::Running(_, _) => {}
        }
        // We know we are running, so we take the handle and wait for it.
        let Mode::Running(_, handle) = std::mem::take(&mut *mode).unwrap() else {
            unreachable!();
        };

        let output = handle.await.expect("run_job should not be able to panic");
        *mode = Some(Mode::Done(output.clone()));
        Ok(output)
    }

    /// Return the data collected from running the job.
    pub async fn data(&self) -> HashMap<String, Value> {
        let mut data = HashMap::new();
        for (&name, state) in self.out.lock().await.iter() {
            if let NodeState::Provided { value } | NodeState::Done { value, .. } = state {
                data.insert(name.to_string(), value.clone());
            }
        }
        data
    }

    pub async fn status(&self) -> HashMap<&'static str, NodeState> {
        self.out.lock().await.clone()
    }
}

/// The current state of a single node in a job.
#[derive(Debug, Clone)]
pub enum NodeState {
    /// Provided by the user when the job was created.
    Provided { value: Value },
    /// Currently being executed.
    Running {
        /// The offset from the job start that this node was started.
        start: Duration,
    },
    /// Job has finished successfully.
    Done {
        /// Time it took to run this node.
        duration: Duration,
        /// Number of retries to finish the node.
        retries: u32,
        /// The output of the node.
        value: Value,
    },
    Retrying {
        /// Current retry start.
        start: Duration,
        /// Retry count.
        retries: u32,
    },
    Failed {
        /// The node failed at this time.
        duration: Duration,
        /// Number of retries attempted for this node.
        retries: u32,
        /// Error returned from the node.
        error: Error,
    },
}

#[allow(clippy::too_many_lines)] // It's okay
async fn run_job<S: State, T: ::std::hash::BuildHasher>(
    job: Job<S>,
    state: S,
    out: Arc<Mutex<HashMap<&'static str, NodeState, T>>>,
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
    let mut pending: HashSet<TypeId> = nodes.keys().copied().collect();

    let mut o = out.lock().await;
    for (id, (name, data)) in job.provided {
        info!(name, "Provided");
        o.insert(
            name,
            NodeState::Provided {
                value: data.clone(),
            },
        );
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
            let start = t0.elapsed();
            let context = ctx(0, start);
            let state = NodeState::Running { start };
            out.lock().await.insert(node.name, state);
            info!(name = node.name, "Node start");
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
            return Output::Done { duration };
        };
        let result = match result {
            Ok(result) => result,
            Err(e) => {
                let duration = t0.elapsed();
                let id = abort_handles[&e.id()];
                let name = nodes[&id].name;
                error!(name, "Node panicked");
                return Output::NodePanic {
                    duration,
                    name,
                    error: format!("{e:?}"),
                };
            }
        };
        match result {
            Node::Done(id, retry, time, Ok(payload)) => {
                results.insert(id, payload.clone());
                let name = nodes[&id].name;
                let state = NodeState::Done {
                    duration: time,
                    retries: retry,
                    value: payload,
                };
                out.lock().await.insert(name, state);
                info!(name, "Node done");
            }
            Node::Done(id, retry, time, Err(e)) => {
                let name = nodes[&id].name;
                if let Some(retry_in) = e.retry_in {
                    warn!(name, retry, error = e.message, ?retry_in, "Node failed");
                    handles.spawn(async move {
                        tokio::time::sleep(time + retry_in).await;
                        Node::Retry(id, retry)
                    });
                } else {
                    let duration = t0.elapsed();
                    let msg = format!("Node {name} failed ({retry} retries): {}", e.message);
                    let state = NodeState::Failed {
                        duration: time,
                        retries: retry,
                        error: e,
                    };
                    out.lock().await.insert(name, state);
                    error!(name, "Node failed");
                    return Output::NodeFailed {
                        duration,
                        name,
                        error: msg,
                    };
                }
            }
            Node::Retry(id, mut retry) => {
                retry += 1;
                let payloads = get_payloads(id);
                let producer = nodes[&id].producer.clone();
                let start = t0.elapsed();
                let context = ctx(retry, start);
                let name = nodes[&id].name;
                let state = NodeState::Retrying {
                    start,
                    retries: retry,
                };
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
