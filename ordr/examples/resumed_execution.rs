use ordr::{Output, build, error, producer, job::Job};

#[derive(Clone, Debug)]
struct Error(&'static str);

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Clone)]
struct Ctx {
    init: usize,
    fail_b: bool,
}

#[derive(Clone, Debug, PartialEq)]
struct A(usize);

#[producer]
async fn make_a(ctx: Ctx) -> Result<A, Error> {
    Ok(A(ctx.init + 1))
}

/// Node B and its producer. Depends on A.
#[derive(Clone, Debug, PartialEq)]
struct B(usize);

#[producer]
async fn make_b(ctx: Ctx, A(a): A) -> Result<B, Error> {
    match ctx.fail_b {
        true => Err(Error("B failed")),
        false => Ok(B(2 + a)),
    }
}

#[derive(Default, Output)]
struct MyResults {
    a: Option<A>,
    b: Option<B>,
}

#[tokio::main]
async fn main() {
    // tracing_subscriber::fmt().init();

    let graph = build!(A, B).unwrap();

    // First execution. It will fail.
    let job = Job::new().with_target::<B>();
    let ctx = Ctx {
        init: 1,
        fail_b: true,
    };

    let error = graph.execute(job, ctx).await.unwrap_err();

    let error::Error::NodeFailed { outputs, .. } = error else {
        panic!("Got unexpected error");
    };

    let results = MyResults::default().with_output_from(&outputs);

    assert_eq!(results.a, Some(A(2)));
    assert_eq!(results.b, None);

    // Here we could serialize and store the results,
    // then later load them up and start again from
    // where we left off.
    let job2 = results.into_job().with_target::<B>();
    let ctx2 = Ctx {
        init: 10,
        fail_b: false,
    };

    let outputs2 = graph.execute(job2, ctx2).await.unwrap();
    let results2 = MyResults::default().with_output_from(&outputs2);

    assert_eq!(results2.a, Some(A(2))); // didn't change because it re-used the value
    assert_eq!(results2.b, Some(B(4))); // this time we got a value
}

/// Ensure that main can run, when running `cargo run --examples`.
#[test]
fn resumed_execution() {
    main();
}
