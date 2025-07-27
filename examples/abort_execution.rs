use ordr::{Context, Error, Job, Worker, producer};
use serde::{Deserialize, Serialize};

#[derive(Clone)]
struct Ctx;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
struct A(usize);

#[producer]
async fn make_a(_ctx: Context<Ctx>) -> Result<A, Error> {
    Ok(A(1))
}

/// Node B and its producer. Depends on A. Fails!
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
struct B(usize);

#[producer]
async fn make_b(_ctx: Context<Ctx>, _a: A) -> Result<B, Error> {
    Err(Error::fatal("B failed"))
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
struct C;

#[producer]
async fn make_c(_ctx: Context<Ctx>, _b: B) -> Result<C, Error> {
    panic!("This will never run")
}

#[tokio::main]
async fn main() {
    // tracing_subscriber::fmt().init();

    let job = Job::builder().add::<C>().build().unwrap();

    let mut worker = Worker::new(job, Ctx);
    worker.run().await.unwrap();
    let output = worker.get_output().await.unwrap();
    assert!(output.is_node_failed());

    let data = worker.data().await;
    let data = serde_json::to_string(&data).unwrap();
    let data_expected = r#"{"A":1}"#;

    assert_eq!(data, data_expected);
}

/// Ensure that main can run, when running `cargo run --examples`.
#[test]
fn abort_execution() {
    main();
}
