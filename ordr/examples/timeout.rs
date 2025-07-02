use std::convert::Infallible;

use ordr::{build, error::Error, job::Job, producer};

#[derive(Clone)]
struct Ctx;

#[derive(Clone, Debug, PartialEq)]
struct A(usize);

#[producer]
async fn make_a(_ctx: Ctx) -> Result<A, Infallible> {
    Ok(A(1))
}

/// Node B and its producer. Depends on A. Fails!
#[derive(Clone, Debug, PartialEq)]
struct B(usize);

#[producer]
async fn make_b(_ctx: Ctx, A(a): A) -> Result<B, Infallible> {
    tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    Ok(B(a + 2))
}

#[tokio::main]
async fn main() {
    let graph = build!(A, B).unwrap();

    let job = Job::new().with_target::<B>();
    let token = job.cancellation_token();

    tokio::spawn(async move {
        tokio::time::sleep(std::time::Duration::from_millis(5)).await;
        token.cancel();
    });

    let error = graph.execute(job, Ctx).await.unwrap_err();

    let Error::Cancelled { outputs, .. } = error else {
        panic!("Got an unexpected error");
    };

    let a = outputs.get::<A>();
    let b = outputs.get::<B>();

    assert_eq!(a, Some(&A(1)));
    assert_eq!(b, None);
}

/// Ensure that main can run, when running `cargo run --examples`.
#[test]
fn timeout() {
    main();
}
