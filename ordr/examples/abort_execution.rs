use ordr::{build, error, executor, job::Job};

#[derive(Clone, Debug)]
struct Error(&'static str);

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::error::Error for Error {}

#[derive(Clone)]
struct Ctx;

#[derive(Clone, Debug, PartialEq)]
struct A(usize);

#[executor]
async fn make_a(_ctx: Ctx) -> Result<A, Error> {
    Ok(A(1))
}

/// Node B and its executor. Depends on A. Fails!
#[derive(Clone, Debug, PartialEq)]
struct B(usize);

#[executor]
async fn make_b(_ctx: Ctx, _a: A) -> Result<B, Error> {
    Err(Error("B failed"))
}

#[derive(Clone, Debug, PartialEq)]
struct C;

#[executor]
async fn make_c(_ctx: Ctx, _b: B) -> Result<C, Error> {
    panic!("This will never run")
}

#[tokio::main]
async fn main() {
    // tracing_subscriber::fmt().init();

    let graph = build!(A, B, C).unwrap();

    let job = Job::new().with_target::<C>();

    let error = graph.execute(job, Ctx).await.unwrap_err();

    let error::Error::NodeFailed { outputs, .. } = error else {
        panic!("Got an unexpected error");
    };

    let a = outputs.get::<A>();
    let b = outputs.get::<B>();
    let c = outputs.get::<C>();

    assert_eq!(a, Some(&A(1)));
    assert_eq!(b, None);
    assert_eq!(c, None);
}

/// Ensure that main can run, when running `cargo run --examples`.
#[test]
fn abort_execution() {
    main();
}
