use std::{collections::HashMap, sync::Arc, time::Duration};

use ordr::{
    Context, Error, Job, NodeBuilder, NodeState, producer, run_job,
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
fn job() {
    let job = Job::builder().add::<B>().build().unwrap();
    assert_eq!(job.len(), 2); // picked up A too
}

#[test]
fn job_with_data() {
    let v = serde_json::to_value(A(1)).unwrap();
    let data = [("A".to_string(), v)].into_iter().collect();
    let job = Job::builder_with_data(data).add::<B>().build().unwrap();
    assert_eq!(job.len(), 1);
}

#[test]
fn job_with_data_from_str() {
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
    let job_fut = run_job(job.clone(), state, out.clone());
    tokio::spawn(job_fut).await.unwrap().unwrap();

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
