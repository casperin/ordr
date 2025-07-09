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
    tracing_subscriber::fmt().init();

    // First execution. It will fail.
    let job = Job::builder().add::<B>().build().unwrap();
    let state = State {
        init: 1,
        fail_b: true,
    };

    let (data, result) = ordr::Worker::run(job, state).await;
    let (name, e) = result.unwrap_err();
    assert_eq!(name, "B");
    assert_eq!(e, "B failed");

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

    let (outputs2, result2) = Worker::run(job2, state2).await;
    result2.unwrap();

    let a = outputs2.get("A").unwrap().as_number().unwrap();
    assert_eq!(*a, Number::from(2));
    let b = outputs2.get("B").unwrap().as_number().unwrap();
    assert_eq!(*b, Number::from(4));
}

/// Ensure that main can run, when running `cargo run --examples`.
#[test]
fn resumed_execution() {
    main();
}
