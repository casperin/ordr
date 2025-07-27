use std::{
    collections::HashMap,
    sync::Arc,
    time::{Duration, Instant},
};

use ordr::{
    Context, Error, Job, NodeBuilder, NodeState, Worker, producer, run_job,
    serde::{Deserialize, Serialize},
    serde_json,
};
use tokio::sync::Mutex;

#[derive(Clone, Debug)]
struct State;

#[derive(Clone, Serialize, Deserialize)]
struct A(u8);

#[producer]
async fn make_a(_ctx: Context<State>) -> Result<A, Error> {
    Ok(A(1))
}

#[derive(Clone, Serialize, Deserialize)]
struct B(u8);

#[producer(name = "BB")]
async fn make_b(_ctx: Context<State>, a: A) -> Result<B, Error> {
    Ok(B(a.0 + 1))
}

#[tokio::test]
async fn producer() {
    let ctx = Context {
        state: State,
        retry: 0,
        start: Duration::from_secs(0),
    };

    // Call A
    let node = A::node();
    let data = (node.producer)(ctx.clone(), vec![]).await.unwrap();
    let A(n): A = serde_json::from_value(data.clone()).unwrap();
    assert_eq!(node.name, "A");
    assert_eq!(n, 1);

    // Call B (with output of A)
    let node = B::node();
    let data = (node.producer)(ctx, vec![data.clone()]).await.unwrap();
    let B(n): B = serde_json::from_value(data).unwrap();
    assert_eq!(node.name, "BB");
    assert_eq!(n, 2);
}

#[test]
fn create_job() {
    let job = Job::builder().add::<B>().build().unwrap();
    assert_eq!(job.len(), 2); // picked up A too
}

#[test]
fn create_job_with_data() {
    let v = serde_json::to_value(A(1)).unwrap();
    let data = [("A".to_string(), v)].into_iter().collect();
    let job = Job::builder_with_data(data).add::<B>().build().unwrap();
    assert_eq!(job.len(), 1);
}

#[test]
fn create_job_with_data_from_str() {
    let json = r#"{"A": 1}"#;
    let data = serde_json::from_str(json).unwrap();
    let job = Job::builder_with_data(data).add::<B>().build().unwrap();
    assert_eq!(job.len(), 1);
}

#[tokio::test]
async fn runs_jobs() {
    let job = Job::builder().add::<B>().build().unwrap();
    let state = State;

    let out = Arc::new(Mutex::new(HashMap::new()));
    let t0 = Instant::now();
    let job_fut = run_job(job.clone(), state, out.clone(), t0);
    tokio::spawn(job_fut).await.unwrap();

    let mut out = out.lock_owned().await;
    let state = out.remove("BB").unwrap();
    match state {
        NodeState::Done(_, _, value) => {
            let B(b): B = serde_json::from_value(value).unwrap();
            assert_eq!(b, 2);
        }
        _ => panic!("Expeced done state"),
    }
}

/// Tests that two jobs actually are started concurrently
#[tokio::test]
async fn concurrent() {
    #[derive(Clone)]
    struct Hist {
        history: Arc<Mutex<Vec<&'static str>>>,
    }
    #[derive(Clone, Serialize, Deserialize)]
    struct A;
    #[derive(Clone, Serialize, Deserialize)]
    struct B;
    #[derive(Clone, Serialize, Deserialize)]
    struct C;

    #[producer]
    async fn a(ctx: Context<Hist>) -> Result<A, Error> {
        ctx.state.history.lock().await.push("start");
        tokio::time::sleep(Duration::from_millis(20)).await;
        ctx.state.history.lock().await.push("end");
        Ok(A)
    }
    #[producer]
    async fn b(ctx: Context<Hist>) -> Result<B, Error> {
        ctx.state.history.lock().await.push("start");
        tokio::time::sleep(Duration::from_millis(20)).await;
        ctx.state.history.lock().await.push("end");
        Ok(B)
    }
    #[producer]
    async fn c(_ctx: Context<Hist>, _: A, _: B) -> Result<C, Error> {
        Ok(C)
    }

    let job = Job::builder().add::<C>().build().unwrap();
    let history = Arc::new(Mutex::new(Vec::new()));
    let hist = Hist {
        history: history.clone(),
    };
    let mut worker = ordr::Worker::new(job, hist);
    worker.run().unwrap();
    let output = worker.get_output().await;
    assert!(output.is_done());
    let hist = history.lock().await;
    assert_eq!(hist[0], "start");
    assert_eq!(hist[1], "start");
    assert_eq!(hist[2], "end");
    assert_eq!(hist[3], "end");
}

#[tokio::test]
async fn output_with_generic() {
    #[derive(Clone, Serialize, Deserialize)]
    struct A<T>(T);

    #[ordr::producer]
    async fn a(_: ordr::Context<()>) -> Result<A<u32>, ordr::Error> {
        Ok(A(22))
    }

    #[derive(Clone, Serialize, Deserialize)]
    struct B;

    type Au32 = A<u32>;

    #[ordr::producer]
    async fn b(_: ordr::Context<()>, _a: Au32) -> Result<B, ordr::Error> {
        Ok(B)
    }

    let job = ordr::Job::builder().add::<B>().build().unwrap();
    let mut worker = ordr::Worker::new(job, ());
    worker.run().unwrap();
    let output = worker.get_output().await;
    assert!(output.is_done());
    let mut data = worker.data().await;
    let A(n): Au32 = serde_json::from_value(data.remove("A").unwrap()).unwrap();
    assert_eq!(n, 22);
}

#[tokio::test]
async fn node_panic() {
    #[derive(Clone, Serialize, Deserialize)]
    struct A;
    #[derive(Clone, Serialize, Deserialize)]
    struct B;
    #[ordr::producer]
    async fn a(_: ordr::Context<()>) -> Result<A, ordr::Error> {
        Ok(A)
    }
    #[ordr::producer(name = "Bomb")]
    async fn b(_: ordr::Context<()>, _: A) -> Result<B, ordr::Error> {
        panic!("boom!")
    }

    let job = ordr::Job::builder().add::<B>().build().unwrap();
    let mut worker = ordr::Worker::new(job, ());
    worker.run().unwrap();
    match worker.get_output().await {
        ordr::Output::NodePanic(_, name, _) => assert_eq!(name, "Bomb"),
        output => panic!("Expected node panic, got {output:?}"),
    }
}

#[tokio::test]
async fn readme_example() {
    #[derive(Clone)]
    struct Ctx {
        // Whatever we need
    }

    // Our node `A`.
    #[derive(Clone, Serialize, Deserialize)]
    struct A(i32);

    #[producer]
    async fn a(_ctx: Context<Ctx>) -> Result<A, Error> {
        // Do some actual work
        Ok(A(123))
    }

    #[derive(Clone, Serialize, Deserialize)]
    struct B(i32);

    #[producer]
    async fn make_b(_ctx: Context<Ctx>, a: A) -> Result<B, Error> {
        Ok(B(a.0 + 2))
    }
    let job = ordr::Job::builder()
        .add::<B>() // Adds `B` and its dependencies recursively
        .build() // Checks that job contains no cycles, etc
        .unwrap();

    // We also need the Context. If your tasks don't need a context, just use `()`.
    let ctx = Ctx {};

    // Next we need a worker to run the job.
    let mut worker = ordr::Worker::new(job, ctx);

    // Start the worker.
    worker.run().unwrap();

    // And get the output once it's done. The output is an enum that you can inspect. It will tell you
    // if a node failed or if the whole job was cancelled, etc.
    let output = worker.get_output().await;

    assert!(output.is_done());

    // Next we can get the collected data/results out. It's a HashMap of name to serialized value.
    let mut data = worker.data().await;

    assert_eq!(data.keys().len(), 2); // Both "A", and "B" is there.

    let b = data.remove("B").unwrap();
    let b: B = serde_json::from_value(b).unwrap();
    assert_eq!(b.0, 125);
}

#[tokio::test]
async fn retrying() {
    #[derive(Clone, Serialize, Deserialize)]
    struct A(u32);

    #[producer]
    async fn a(ctx: Context<()>) -> Result<A, Error> {
        if ctx.retry < 3 {
            let msg = format!("Boom {}", ctx.retry);
            let retry_in = Duration::from_millis(10);
            return Err(Error::with_retry(msg, retry_in));
        }
        Ok(A(ctx.retry))
    }

    let job = Job::builder().add::<A>().build().unwrap();
    let mut worker = Worker::new(job, ());
    worker.run().unwrap();
    let output = worker.get_output().await;
    assert!(output.is_done());
    let v = worker.data().await.remove("A").unwrap();
    let a = serde_json::from_value::<A>(v).unwrap();
    assert_eq!(a.0, 3);
}
