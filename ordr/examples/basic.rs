use std::convert::Infallible;

use ordr::{build, executor, job::Job};

/// Basic Ctx for the nodes. You can put anything in it, as long as it implements `Clone`.
#[derive(Clone)]
struct Ctx;

/// Node A and its executor.
#[derive(Clone, Debug)]
struct A(usize);

#[executor]
async fn make_a(_ctx: Ctx) -> Result<A, Infallible> {
    Ok(A(1))
}

/// Node B and its executor. Depends on A.
#[derive(Clone, Debug)]
struct B(usize);

#[executor]
async fn make_b(_ctx: Ctx, A(a): A) -> Result<B, Infallible> {
    Ok(B(2 + a))
}

/// Node C and its executor. Depends on A.
#[derive(Clone, Debug)]
struct C(usize);

#[executor]
async fn make_c(_ctx: Ctx, A(a): A) -> Result<C, Infallible> {
    Ok(C(3 + a))
}

/// Node D and its executor. Depends on B and C.
#[derive(Clone, Debug, PartialEq)]
struct D(usize);

#[executor]
async fn make_d(_ctx: Ctx, B(b): B, C(c): C) -> Result<D, Infallible> {
    Ok(D(4 + b + c))
}

#[tokio::main]
async fn main() {
    // tracing_subscriber::fmt().init();

    // Create a graph. It is diamond shaped.
    let graph = build!(A, B, C, D).unwrap();

    // Create a job, that has the D as the target
    let job = Job::new().with_target::<D>();

    // Dummy Ctx
    let ctx = Ctx;

    // Execute the job
    let outputs = graph.execute(job, ctx).await.unwrap();

    // Get the D out of the outputs
    let d = outputs.get::<D>();

    assert_eq!(d, Some(&D(11)));
}

#[cfg(test)]
mod tests {
    /// Ensure that main can run, when running `cargo run --examples`.
    #[test]
    fn basic() {
        super::main();
    }
}
