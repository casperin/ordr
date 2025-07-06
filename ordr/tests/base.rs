use std::{collections::HashMap, time::Duration};

use ordr::{
    Context, Error, NodeBuilder, new_job, producer, run_job,
    serde::{Deserialize, Serialize},
};
use tokio::sync::mpsc::channel;

#[derive(Clone, Debug)]
struct State;

#[derive(Clone, Serialize, Deserialize)]
struct A(u8);

#[producer(output=A, state=State)]
async fn make_a(_ctx: Context<State>) -> Result<A, Error> {
    Ok(A(1))
}

#[derive(Clone, Serialize, Deserialize)]
struct B(u8);

#[producer(name="BB", output=B, state=State)]
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
    let A(n): A = ordr::serde_cbor::from_slice(&data).unwrap();
    assert_eq!(node.name, "A");
    assert_eq!(n, 1);

    // Call B (with output of A)
    let node = B::node();
    let data = (node.producer)(ctx, vec![data]).await.unwrap();
    let B(n): B = ordr::serde_cbor::from_slice(&data).unwrap();
    assert_eq!(node.name, "BB");
    assert_eq!(n, 2);
}

#[test]
fn aaa_job() {
    let job = new_job!(B).unwrap();
    assert_eq!(job.len(), 2); // picked up A too
}

#[tokio::test]
async fn aaa_run_job() {
    let job = new_job!(B).unwrap();
    let state = State;
    let results = HashMap::new();
    let (tx, mut rx) = channel(2);
    let job2 = job.clone();

    let handle = tokio::spawn(async move {
        run_job(job2, state, results, tx).await;
    });

    // Our results
    let mut results = HashMap::new();
    while let Some(msg) = rx.recv().await {
        if let ordr::Msg::NodeDone(id, _, data) = msg {
            let name = job.name(&id);
            results.insert(name, data);
        }
    }

    handle.await.unwrap();

    let A(a): A = ordr::serde_cbor::from_slice(&results["A"]).unwrap();
    let B(b): B = ordr::serde_cbor::from_slice(&results["BB"]).unwrap();
    assert_eq!(a, 1);
    assert_eq!(b, 2);
}
