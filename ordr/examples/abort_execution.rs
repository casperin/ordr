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
    Err(Error {
        message: "B failed".into(),
        retry_in: None,
    })
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

    let (data, result) = Worker::run(job, Ctx).await;
    result.unwrap_err();

    let data = serde_json::to_string(&data).unwrap();
    let data_expected = r#"{"A":1}"#;

    assert_eq!(data, data_expected);
}

/// Ensure that main can run, when running `cargo run --examples`.
#[test]
fn abort_execution() {
    main();
}
