use ordr::{Context, Error, Job, Worker, producer};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
struct A;
#[derive(Debug, Clone, Serialize, Deserialize)]
struct B;
#[derive(Debug, Clone, Serialize, Deserialize)]
struct C;
#[derive(Debug, Clone, Serialize, Deserialize)]
struct D;
#[derive(Debug, Clone, Serialize, Deserialize)]
struct E;

#[producer]
async fn a(_: Context<()>) -> Result<A, Error> {
    wait("A", "31", 0).await;
    Ok(A)
}

#[producer]
async fn b(_: Context<()>, _: A) -> Result<B, Error> {
    wait("B", "32", 15).await;
    Ok(B)
}

#[producer]
async fn c(_: Context<()>, _: A) -> Result<C, Error> {
    wait("C", "33", 40).await;
    Ok(C)
}

#[producer]
async fn d(_: Context<()>, _: B) -> Result<D, Error> {
    wait("D", "35", 15).await;
    Ok(D)
}

#[producer]
async fn e(_: Context<()>, _: C, _: D) -> Result<E, Error> {
    wait("E", "36", 0).await;
    Ok(E)
}

async fn wait(node: &'static str, color: &'static str, timeout: u64) {
    let d = std::time::Duration::from_millis(5);
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
//   A
//  / \
// B   C
// |   |
// D   |
//  \ /
//   E

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt().init();
    let job = Job::builder().add::<E>().build().unwrap();
    // println!("{}", ordr::mermaid(&job));
    let mut worker = Worker::new(job, ());
    worker.run().await.unwrap();
    let output = worker.get_output().await.unwrap();
    assert!(output.is_done());
}
