use std::{convert::Infallible, time::Duration};

use ordr::{build, job::Job, producer};

#[derive(Debug, Clone)]
struct A;
#[derive(Debug, Clone)]
struct B;
#[derive(Debug, Clone)]
struct C;
#[derive(Debug, Clone)]
struct D;
#[derive(Debug, Clone)]
struct E;

#[producer]
async fn a(_: ()) -> Result<A, Infallible> {
    wait("A", "31", 0).await;
    Ok(A)
}

#[producer]
async fn b(_: (), _: A) -> Result<B, Infallible> {
    wait("B", "32", 15).await;
    Ok(B)
}

#[producer]
async fn c(_: (), _: A) -> Result<C, Infallible> {
    wait("C", "33", 40).await;
    Ok(C)
}

#[producer]
async fn d(_: (), _: B) -> Result<D, Infallible> {
    wait("D", "35", 15).await;
    Ok(D)
}

#[producer]
async fn e(_: (), _: C, _: D) -> Result<E, Infallible> {
    wait("E", "36", 0).await;
    Ok(E)
}

async fn wait(node: &'static str, color: &'static str, timeout: u64) {
    let d = Duration::from_millis(5);
    let t = std::time::Instant::now();
    let x = std::time::Duration::from_millis(timeout);

    println!("\x1b[{color}m{node} start\x1b[0m");

    for n in 1.. {
        tokio::time::sleep(d).await;
        if t.elapsed() >= x {
            println!("\x1b[{color}m{node} end\x1b[0m");
            return;
        }
        println!("\x1b[{color}m{node} {n}\x1b[0m");
    }
}

// This is the graph. B + D is faster than C.
//
//  .A
//  / \
// B   C
// |   |
// D   |
//  \ /
//   E

#[tokio::main]
async fn main() {
    let graph = build!(A, B, C, D, E).unwrap();
    let job = Job::new().with_target::<E>();
    println!("{}", graph.mermaid(&job));
    graph.execute(job, ()).await.unwrap();
}
