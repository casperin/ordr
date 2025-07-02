use std::fmt::Display;

use ordr::{build, error::Error, executor, job::Job};

#[derive(Clone)]
#[allow(unused)]
struct Ctx(u8);

#[derive(Debug)]
struct E;

impl Display for E {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Error")
    }
}

macro_rules! node {
    ($name:ident: $ty:ident, $ret:expr) => {
        #[derive(Clone, Debug, PartialEq, Eq)]
        pub(crate) struct $ty;

        #[executor]
        async fn $name(_ctx: Ctx) -> Result<$ty, E> {
            $ret
        }
    };

    ($name:ident: $ty:ident, $ret:expr, $($dep:ident),*) => {
        #[derive(Clone, Debug, PartialEq, Eq)]
        pub(crate) struct $ty;

        #[executor]
        async fn $name(_ctx: Ctx, $( _: $dep ),*) -> Result<$ty, E> {
            $ret
        }
    };
}

mod basic {
    use ordr::Output;

    use super::*;

    node!(a: A, Ok(A));
    node!(b: B, Ok(B), A);

    #[derive(Output, Default)]
    struct MyOutput {
        a: Option<A>,
        b: Option<B>,
    }

    #[tokio::test]
    async fn basic() {
        let graph = build!(A, B).unwrap();
        let job = Job::new().with_target::<B>();
        let outputs = graph.execute(job, Ctx(0)).await.unwrap();
        let my_output = MyOutput::default().with_output_from(&outputs);

        assert_eq!(outputs.get::<A>(), Some(&A));
        assert_eq!(outputs.get::<B>(), Some(&B));

        assert_eq!(my_output.a, Some(A));
        assert_eq!(my_output.b, Some(B));
    }
}

mod parse_attr {
    use ordr::node::NodeBuilder;

    use super::*;

    type R<T> = Result<T, E>;

    #[derive(Clone)]
    struct A;

    #[executor(name = "OtherName", output = A, error = E)]
    async fn a(_: ()) -> R<A> {
        Ok(A)
    }

    #[test]
    fn parse_attr() {
        let node: ordr::node::Node<(), _> = A::node();
        assert_eq!(node.name, "OtherName");
    }
}

mod validate_nodes {
    use super::*;

    #[test]
    fn detect_cycle() {
        node!(a: A, Ok(A), B);
        node!(b: B, Ok(B), C);
        node!(c: C, Ok(C), A);

        let r = build!(A, B, C);
        assert!(matches!(r, Err(Error::Cycle(_))));
    }

    #[test]
    fn detect_empty_graph() {
        let r: Result<ordr::graph::Graph<(), E>, Error<E>> = build!();
        assert!(matches!(r, Err(Error::NoNodes)));
    }

    #[test]
    fn detect_missing_dependency() {
        node!(a: A, Ok(A));
        node!(b: B, Ok(B), A);
        node!(c: C, Ok(C), B);

        let r = build!(B, C);
        assert!(matches!(r, Err(Error::DependencyNotFound("B", _))));
    }
}

mod validate_job {
    use super::*;

    #[test]
    fn target_not_found() {
        node!(a: A, Ok(A));
        node!(b: B, Ok(B), A);
        node!(c: C, Ok(C), B);
        node!(d: D, Ok(D), C);

        let graph = build!(A, B, C).unwrap(); // D missing
        let job = Job::new().with_target::<D>();

        let r = graph.validate_job(&job);
        assert!(matches!(r, Err(Error::NodeNotFound(_))));
    }
}

mod node_errors {
    use super::*;

    #[tokio::test]
    async fn node_failed() {
        node!(a: A, Ok(A));
        node!(b: B, Ok(B), A);
        node!(c: C, Err(E), B);
        node!(d: D, Ok(D), C);

        let graph = build!(A, B, C, D).unwrap();
        let job = Job::new().with_target::<D>();
        graph.validate_job(&job).unwrap();
        let r = graph.execute(job, Ctx(0)).await;
        assert!(matches!(r, Err(Error::NodeFailed { .. })));
    }

    #[tokio::test]
    async fn node_panic() {
        node!(a: A, Ok(A));
        node!(b: B, Ok(B), A);
        node!(c: C, panic!("C"), B);
        node!(d: D, Ok(D), C);

        let graph = build!(A, B, C, D).unwrap();
        let job = Job::new().with_target::<D>();
        graph.validate_job(&job).unwrap();
        let r = graph.execute(job, Ctx(0)).await;
        assert!(matches!(r, Err(Error::NodePanic { .. })));
    }
}

mod cancelled {
    use super::*;

    #[tokio::test]
    async fn cancelled() {
        #[derive(Clone, Debug, PartialEq, Eq)]
        struct C;

        #[executor]
        async fn c(_: Ctx, _: B) -> Result<C, E> {
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
            Ok(C)
        }

        node!(a: A, Ok(A));
        node!(b: B, Ok(B), A);
        node!(d: D, Ok(D), C);

        let graph = build!(A, B, C, D).unwrap();
        let job = Job::new().with_target::<D>();
        let token = job.cancellation_token();

        tokio::spawn(async move {
            tokio::time::sleep(std::time::Duration::from_millis(20)).await;
            token.cancel();
        });

        let r = graph.execute(job, Ctx(0)).await;
        let outputs = match r {
            Err(Error::Cancelled { outputs }) => outputs,
            _ => panic!("Got bad response: {r:?}"),
        };

        assert_eq!(outputs.get::<A>(), Some(&A));
        assert_eq!(outputs.get::<B>(), Some(&B));
        assert_eq!(outputs.get::<C>(), None);
        assert_eq!(outputs.get::<D>(), None);
    }
}

mod error_type {
    use anyhow::bail;

    use super::*;

    #[test]
    fn thiserror() {
        #[derive(Debug, thiserror::Error)]
        enum E {
            #[error("Hello error")]
            BadError,
        }

        #[derive(Clone)]
        struct A;

        #[executor]
        async fn make_a(_: ()) -> Result<A, E> {
            Err(E::BadError)
        }
    }

    #[test]
    fn anyhow() {
        #[derive(Clone)]
        struct A;

        #[executor]
        async fn make_a(_: ()) -> Result<A, anyhow::Error> {
            bail!("Oh no!")
        }
    }
}

mod input {
    use super::*;

    #[tokio::test]
    async fn add_input_to_job() {
        node!(a: A, Err(E)); // We can't use this executor to create an A
        node!(b: B, Ok(B), A);
        let graph = build!(A, B).unwrap();
        let job = Job::new().with_target::<B>().with_input(A); // We insert A manually
        let output = graph.execute(job, Ctx(0)).await.unwrap();
        assert!(output.get::<A>().is_some());
        assert!(output.get::<B>().is_some());
    }
}

mod resume_job {
    use ordr::Output;

    use super::*;

    #[tokio::test]
    async fn resume_cancelled_job() {
        #[derive(Clone, Debug, PartialEq, Eq)]
        struct A(u8);
        #[derive(Clone, Debug, PartialEq, Eq)]
        struct B(u8);
        #[derive(Clone, Debug, PartialEq, Eq)]
        struct C(u8);

        #[executor]
        async fn a(ctx: u8) -> Result<A, E> {
            Ok(A(ctx + 1))
        }

        #[executor]
        async fn b(ctx: u8, A(a): A) -> Result<B, E> {
            match ctx {
                0 => Err(E),
                n => Ok(B(n + a + 1)),
            }
        }

        #[executor]
        async fn c(ctx: u8, B(b): B) -> Result<C, E> {
            Ok(C(ctx + b + 1))
        }

        let graph = build!(A, B, C).unwrap();
        let job = Job::new().with_target::<C>();

        // The Ctx=0 makes B abort mission
        let r = graph.execute(job, 0).await;

        let outputs = match r {
            Err(Error::NodeFailed { outputs, .. }) => outputs,
            _ => panic!("Got bad response: {r:?}"),
        };

        // Ensure we didn't finish the job
        assert_eq!(outputs.get::<C>(), None);

        #[derive(Output, Default, PartialEq, Eq, Debug)]
        struct Mine {
            a: Option<A>,
            b: Option<B>,
            c: Option<C>,
        }

        let mine = Mine::default().with_output_from(&outputs);
        let mine_expected = Mine {
            a: Some(A(1)),
            b: None,
            c: None,
        };
        assert_eq!(mine, mine_expected);

        // We are certain that we have an unfinished output. Let's try again,
        // but this time without aborting.
        let job = mine.into_job().with_target::<C>();
        let output = graph.execute(job, 10).await.unwrap();

        // Didn't change, because it wasn't run again (it'd be 11 if so).
        assert_eq!(output.get::<A>(), Some(&A(1))); //  0 + 1
        assert_eq!(output.get::<B>(), Some(&B(12))); // 10 + 1 + 1
        assert_eq!(output.get::<C>(), Some(&C(23))); // 10 + 12 + 1
    }
}

mod concurrent {
    use std::{sync::Arc, time::Duration};

    use tokio::{sync::Mutex, time::sleep};

    use super::*;

    #[derive(Clone, Default)]
    struct Ctx {
        events: Arc<Mutex<Vec<&'static str>>>,
    }

    #[derive(Clone, Debug, PartialEq, Eq)]
    struct A;
    #[derive(Clone, Debug, PartialEq, Eq)]
    struct B;
    #[derive(Clone, Debug, PartialEq, Eq)]
    struct C;

    #[executor]
    async fn a(_: Ctx) -> Result<A, E> {
        Ok(A)
    }

    #[executor]
    async fn b(ctx: Ctx, _: A) -> Result<B, E> {
        ctx.events.lock().await.push("start");
        sleep(Duration::from_millis(20)).await;
        ctx.events.lock().await.push("end");
        Ok(B)
    }

    #[executor]
    async fn c(ctx: Ctx, _: A) -> Result<C, E> {
        ctx.events.lock().await.push("start");
        sleep(Duration::from_millis(20)).await;
        ctx.events.lock().await.push("end");
        Ok(C)
    }

    #[tokio::test]
    async fn runs_nodes_concurrently() {
        let graph = build!(A, B, C).unwrap();
        let job = Job::new().with_target::<B>().with_target::<C>();
        let ctx = Ctx::default();
        graph.execute(job, ctx.clone()).await.unwrap();

        let events = ctx.events.lock().await;
        let events_expected = vec!["start", "start", "end", "end"];
        assert_eq!(*events, events_expected);
    }
}

mod split_mods {
    use super::*;

    mod x {
        use super::*;
        node!(a: A, Ok(A));
    }

    mod y {
        use super::*;
        use x::A;
        node!(b: B, Ok(B), A);
    }

    #[test]
    fn from_different_mods() {
        build!(x::A, y::B).unwrap();
    }
}
