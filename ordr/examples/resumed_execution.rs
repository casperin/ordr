use ordr::{Context, Error, Job, Worker, producer};
use serde::{Deserialize, Serialize};
use serde_json::Number;

#[derive(Clone)]
struct State {
    init: usize,
    fail_b: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
struct A(usize);

#[producer]
async fn make_a(ctx: Context<State>) -> Result<A, Error> {
    Ok(A(ctx.state.init + 1))
}

/// Node B and its producer. Depends on A.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
struct B(usize);

#[producer]
async fn make_b(ctx: Context<State>, A(a): A) -> Result<B, Error> {
    match ctx.state.fail_b {
        true => Err(Error {
            message: "B failed".into(),
            retry_in: None,
        }),
        false => Ok(B(2 + a)),
    }
}

#[tokio::main]
async fn main() {
    // First execution. It will fail.
    let job = Job::builder().add::<B>().build().unwrap();
    let state = State {
        init: 1,
        fail_b: true,
    };

    let mut worker = Worker::new(job, state);
    worker.run().unwrap();
    let e = worker.wait_for_job().await.unwrap_err();
    assert!(e.contains("B failed"));
    let data = worker.data().await;

    let json = serde_json::to_string(&data).unwrap();
    let json_expected = r#"{"A":2}"#;
    assert_eq!(json, json_expected);

    // Restart with our json
    let data = serde_json::from_str(&json).unwrap();
    let job2 = Job::builder_with_data(data).add::<B>().build().unwrap();
    let state2 = State {
        init: 10,
        fail_b: false,
    };

    let mut worker = Worker::new(job2, state2);
    worker.run().unwrap();
    worker.wait_for_job().await.unwrap();
    let data = worker.data().await;

    let a = data.get("A").unwrap().as_number().unwrap();
    assert_eq!(*a, Number::from(2));
    let b = data.get("B").unwrap().as_number().unwrap();
    assert_eq!(*b, Number::from(4));
}

/// Ensure that main can run, when running `cargo run --examples`.
#[test]
fn resumed_execution() {
    main();
}
