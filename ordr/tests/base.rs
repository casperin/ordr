use std::{collections::HashMap, time::Duration};

use ordr::{
    Context, Error, Job, NodeBuilder, producer, run_job,
    serde::{Deserialize, Serialize},
    serde_json,
};
use tokio::sync::mpsc::channel;

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
async fn aaa_producer() {
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
fn aaa_job() {
    let job = Job::builder().add::<B>().build().unwrap();
    assert_eq!(job.len(), 2); // picked up A too
}

#[test]
fn aaa_job_with_data() {
    let v = serde_json::to_value(A(1)).unwrap();
    let data = [("A".to_string(), v)].into_iter().collect();
    let job = Job::builder_with_data(data).add::<B>().build().unwrap();
    assert_eq!(job.len(), 1);
}

#[test]
fn aaa_job_with_data_from_str() {
    let json = r#"{"A": 1}"#;
    let data = serde_json::from_str(json).unwrap();
    let job = Job::builder_with_data(data).add::<B>().build().unwrap();
    assert_eq!(job.len(), 1);
}

#[tokio::test]
async fn aaa_run_job() {
    let job = Job::builder().add::<B>().build().unwrap();
    let state = State;
    let (tx, mut rx) = channel(2);

    let job_fut = run_job(job.clone(), state, tx);
    let handle = tokio::spawn(job_fut);

    let mut results = HashMap::new();
    while let Some(msg) = rx.recv().await {
        if let ordr::Msg::NodeDone(name, _, data) = msg {
            results.insert(name, data);
        }
    }

    handle.await.unwrap();

    let A(a): A = serde_json::from_value(results.remove("A").unwrap()).unwrap();
    let B(b): B = serde_json::from_value(results.remove("BB").unwrap()).unwrap();
    assert_eq!(a, 1);
    assert_eq!(b, 2);
}
