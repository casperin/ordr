#![allow(unused)]

use std::convert::Infallible;

use ordr::{build_graph, executor, job::Job};

#[derive(Debug, Clone)]
struct Trees(usize);

#[executor]
async fn trees(_: ()) -> Result<Trees, Infallible> {
    Ok(Trees(3))
}

#[derive(Debug, Clone)]
struct Friends(Vec<&'static str>);

#[executor]
async fn make_friends(_: ()) -> Result<Friends, Infallible> {
    Ok(Friends(vec!["Paul", "Sarah", "Ida"]))
}

#[derive(Debug, Clone)]
struct Money(usize);

#[executor]
async fn work(_: ()) -> Result<Money, Infallible> {
    Ok(Money(152))
}

#[derive(Debug, Clone)]
struct Paper(usize);

#[executor]
async fn chop_trees(_: (), _trees: Trees, _friends: Friends) -> Result<Paper, Infallible> {
    Ok(Paper(152))
}

#[derive(Debug, Clone)]
struct Ideas(Vec<&'static str>);

#[executor]
async fn travel(_: (), _money: Money) -> Result<Ideas, Infallible> {
    Ok(Ideas(vec!["Cats are great", "Dogs too"]))
}

#[derive(Debug, Clone)]
struct GetRich(usize);

#[executor]
async fn write_amazing_book(_: (), _paper: Paper, _ideas: Ideas) -> Result<GetRich, Infallible> {
    Ok(GetRich(152))
}

#[derive(Debug, Clone)]
struct HaveFun(bool);

#[executor]
async fn play(_: (), _friends: Friends, _ideas: Ideas) -> Result<HaveFun, Infallible> {
    Ok(HaveFun(true))
}

fn main() {
    let graph = build_graph!(Trees, Friends, Money, Paper, Ideas, GetRich, HaveFun).unwrap();

    let job = Job::new()
        .with_input(Friends(vec!["Paul", "Sarah", "Ida"]))
        .with_input(Money(32))
        .with_target::<HaveFun>();

    let diagram = graph.mermaid(&job);
    println!("{diagram}");
}
