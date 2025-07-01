use std::fmt::Display;

use ordr::{build, error::Error, executor, job::Job};

#[derive(Clone)]
#[allow(unused)]
struct Ctx(u8);

#[derive(Debug)]
struct E;

impl std::error::Error for E {}

impl Display for E {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Error")
    }
}

macro_rules! node {
    ($name:ident: $ty:ident, $t:expr, $ret:expr) => {
        #[derive(Clone, Debug, PartialEq, Eq)]
        struct $ty;

        #[executor]
        async fn $name(_ctx: Ctx) -> Result<$ty, E> {
            tokio::time::sleep(std::time::Duration::from_millis($t)).await;
            $ret
        }
    };

    ($name:ident: $ty:ident, $t:expr, $ret:expr, $($dep:ident),*) => {
        #[derive(Clone, Debug, PartialEq, Eq)]
        struct $ty;

        #[executor]
        async fn $name(_ctx: Ctx, $( _: $dep ),*) -> Result<$ty, E> {
            tokio::time::sleep(std::time::Duration::from_millis($t)).await;
            $ret
        }
    };
}

mod basic {
    use ordr::Output;

    use super::*;

    node!(a: A, 0, Ok(A));
    node!(b: B, 0, Ok(B), A);

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
        node!(a: A, 0, Ok(A), B);
        node!(b: B, 0, Ok(B), C);
        node!(c: C, 0, Ok(C), A);

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
        node!(a: A, 0, Ok(A));
        node!(b: B, 0, Ok(B), A);
        node!(c: C, 0, Ok(C), B);

        let r = build!(B, C);
        assert!(matches!(r, Err(Error::DependencyNotFound("B", _))));
    }
}

mod validate_job {
    use super::*;

    #[test]
    fn target_not_found() {
        node!(a: A, 0, Ok(A));
        node!(b: B, 0, Ok(B), A);
        node!(c: C, 0, Ok(C), B);
        node!(d: D, 0, Ok(D), C);

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
        node!(a: A, 0, Ok(A));
        node!(b: B, 0, Ok(B), A);
        node!(c: C, 0, Err(E), B);
        node!(d: D, 0, Ok(D), C);

        let graph = build!(A, B, C, D).unwrap();
        let job = Job::new().with_target::<D>();
        graph.validate_job(&job).unwrap();
        let r = graph.execute(job, Ctx(0)).await;
        assert!(matches!(r, Err(Error::NodeFailed { .. })));
    }

    #[tokio::test]
    async fn node_panic() {
        node!(a: A, 0, Ok(A));
        node!(b: B, 0, Ok(B), A);
        node!(c: C, 0, panic!("C"), B);
        node!(d: D, 0, Ok(D), C);

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
        node!(a: A, 0, Ok(A));
        node!(b: B, 0, Ok(B), A);
        node!(c: C, 100, Ok(C), B);
        node!(d: D, 0, Ok(D), C);

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
