use std::convert::Infallible;

use ordr::{build, producer, job::Job};

#[derive(Clone)]
struct Ctx;

#[derive(Clone, Debug)]
struct A(usize);

#[producer]
async fn make_a(_ctx: Ctx) -> Result<A, Infallible> {
    Ok(A(1))
}

/// Node B and its producer. Depends on A.
#[derive(Clone, Debug, PartialEq)]
struct B(usize);

#[producer]
async fn make_b(_ctx: Ctx, A(a): A) -> Result<B, Infallible> {
    Ok(B(2 + a))
}

#[tokio::main]
async fn main() {
    let graph = build!(A, B).unwrap();

    // Create a job, where we provide an `A`.
    // This means that `make_a` will never run.
    let job = Job::new().with_input(A(10)).with_target::<B>();

    let outputs = graph.execute(job, Ctx).await.unwrap();

    let b = outputs.get::<B>();

    // B has the result of 10+2, not 1+2.
    assert_eq!(b, Some(&B(12)));
}

/// Ensure that main can run, when running `cargo run --examples`.
#[test]
fn provide_dependencies() {
    main();
}
