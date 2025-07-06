use std::any::TypeId;

use ordr::{
    graph::Graph,
    node::{Context, Error, Node},
    producer,
};

#[derive(Clone)]
struct State;

#[derive(Clone)]
struct A;

#[producer]
async fn a(_ctx: Context<State>) -> Result<A, Error> {
    Ok(A)
}

#[derive(Clone)]
struct B;

#[producer(name = "BB")]
async fn b(_ctx: Context<State>, _a: A) -> Result<B, Error> {
    Ok(B)
}

#[test]
fn aaa_node_name() {
    assert_eq!(A::name(), "A");
    assert_eq!(B::name(), "BB");
}

#[test]
fn aaa_graph_building() {
    Graph::builder().node::<A>().node::<B>().build().unwrap();
    Graph::builder().node::<B>().build().unwrap_err();
}

#[test]
fn aaa_task_building() {
    let g = Graph::builder().node::<A>().node::<B>().build().unwrap();

    g.task_builder().target("BB").unwrap();
    g.task_builder().targets(&["A", "BB"]).unwrap();
    if g.task_builder().target("C").is_ok() {
        panic!("This should fail because C isn't in the graph");
    }
}
