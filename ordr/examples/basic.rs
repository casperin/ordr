use ordr::{Context, Error, Job, Worker, producer};
use serde::{Deserialize, Serialize};

/// Basic Ctx for the nodes. You can put anything in it, as long as it implements `Clone`.
#[derive(Clone)]
struct Ctx;

/// Node A and its producer.
#[derive(Clone, Debug, Serialize, Deserialize)]
struct A(usize);

#[producer]
async fn make_a(_ctx: Context<Ctx>) -> Result<A, Error> {
    Ok(A(1))
}

/// Node B and its producer. Depends on A.
#[derive(Clone, Debug, Serialize, Deserialize)]
struct B(usize);

#[producer]
async fn make_b(_ctx: Context<Ctx>, A(a): A) -> Result<B, Error> {
    Ok(B(2 + a))
}

/// Node C and its producer. Depends on A.
#[derive(Clone, Debug, Serialize, Deserialize)]
struct C(usize);

#[producer]
async fn make_c(_ctx: Context<Ctx>, A(a): A) -> Result<C, Error> {
    Ok(C(3 + a))
}

/// Node D and its producer. Depends on B and C.
#[derive(Clone, Debug, Serialize, Deserialize)]
struct D(usize);

#[producer]
async fn make_d(_ctx: Context<Ctx>, B(b): B, C(c): C) -> Result<D, Error> {
    Ok(D(4 + b + c))
}

#[tokio::main]
async fn main() {
    // tracing_subscriber::fmt().init();

    // Create a job, that has the D as the target
    let job = Job::builder().add::<D>().build().unwrap();

    // Dummy Ctx
    let ctx = Ctx;

    // Execute the job
    let mut job = Worker::new(job, ctx);
    job.run().unwrap();
    job.wait_for_job().await.unwrap();
    let data = job.data().await;

    // Get the D out of the outputs
    let data = serde_json::to_value(data).unwrap();
    let output: Output = serde_json::from_value(data).unwrap();
    let output_expected = Output {
        a: 1,
        b: 3,
        c: 4,
        d: 11,
    };
    assert_eq!(output, output_expected);
}

#[derive(Debug, Deserialize, PartialEq)]
#[serde(rename_all = "UPPERCASE")]
struct Output {
    a: i32,
    b: i32,
    c: i32,
    d: i32,
}

#[cfg(test)]
mod tests {
    /// Ensure that main can run, when running `cargo run --examples`.
    #[test]
    fn basic() {
        super::main();
    }
}
