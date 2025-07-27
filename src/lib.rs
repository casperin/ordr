#![warn(missing_docs)]

//! Ordr is a library that helps you execute and keep track of a set of interdependent functions.
//!
//! It can create a graph (specifically a `DAG`) of functions depending on functions, and execute them as they get ready, in parallel.
//!
//! Here is a simple example taken from one of the examples in the repo:
//!
//! ```txt
//! ╭─────> C ────╮
//! A             ├─> E
//! ╰──> B ──> D ─╯
//! ```
//!
//! `E` is our target, and it depends `C` and `D` and so forth. Ordr will thus start executing `A`,
//! when that's done, it will execute `B` and `C` in parallel with the output of `A`, once `B` is done,
//! it'll start `D` (with the output of `B`) and when ready, `E` will be executed.
//!
//! If any of the tasks return an error, the running tasks will be aborted and the execution stops and
//! a partial output will be returned.
//!
//! A job (such as the above) can also be started with already existing data. Say in the above `C`
//! fails after `B` and `D` have completed successfully, we can then run it again with the `A`, `B`,
//! and `D` data, which will result in only `C` and then `E` being run.
//!
//! The letters in the graph, we call nodes. In Rust code, they can be any struct, and they are the
//! output of a [`producer`]; an `async` function that takes a [`Context`], any number of other nodes,
//! and returns a [`Result<A>`]. The context, contains a `state` that can be anything you want as
//! long as it implements `Clone`. It's meant to be used for having database connections or
//! whatever else you need.
//!
//! It looks like this:
//!
//! ```
//! # async {
//! use serde::{Deserialize, Serialize};
//!
//! #[derive(Clone)]
//! struct State {
//!     // Whatever we need
//! }
//!
//! // Our node `A`.
//! #[derive(Clone, Serialize, Deserialize)]
//! struct A(i32);
//!
//! #[ordr::producer]
//! async fn my_a_producer(_ctx: ordr::Context<State>) -> ordr::Result<A> {
//!     // Do some actual work
//!     Ok(A(123))
//! }
//!
//! // If we then have a node `B` that depends on `A`, we just add it to the arguments:
//!
//! #[derive(Clone, Serialize, Deserialize)]
//! struct B(i32);
//!
//! #[ordr::producer]
//! async fn make_b(_ctx: ordr::Context<State>, a: A) -> ordr::Result<B> {
//!     Ok(B(a.0 + 2))
//! }
//!
//! // Before we start executing anything, we need to make a job. The `.add::<T>()` adds the node
//! // and all its dependencies (in this case `A`).
//! //
//! // You can add as many targets as you'd like.
//! let job = ordr::Job::builder().add::<B>().build().unwrap();
//!
//! // We also need the Context. If your tasks don't need a context, just use `()`.
//! let state = State {};
//!
//! // Next we need a worker to execute the job.
//! let mut worker = ordr::Worker::new(job, state);
//!
//! // Start the worker.
//! worker.run().await.unwrap();
//!
//! // And get the output once it's done. The output is an enum that you can inspect. It will tell
//! // you if a node failed or if the whole job was cancelled, etc.
//! let output = worker.get_output().await.unwrap();
//!
//! assert!(output.is_done());
//!
//! // Next we can get the collected data/results out. It's a HashMap of the name of the node
//! // (struct name) to serialized value.
//! let mut data = worker.data().await;
//!
//! assert_eq!(data.keys().len(), 2); // Both "A", and "B" is there.
//!
//! let b = data.remove("B").unwrap();
//! let b: B = serde_json::from_value(b).unwrap();
//! assert_eq!(b.0, 125);
//! # };
//! ```
//!
//! A few things to keep in mind:
//!
//! * All nodes and the context must implement `Clone` and Serde's `Serialize` and `Deserialize`.
//! * All producers must return a `ordr::Result` (which is a `Result<T, ordr::Error>`.
//! * All producers must be async and take `ordr::Context<State>` as the first parameter.
//!     * `State` is your state. Whatever you need.
//!
//!
//! # Mermaid diagram
//!
//! It might be useful to inspect a `Job` visually. You can get a graph like this:
//!
//! ```
//! # #[derive(Clone, serde::Serialize, serde::Deserialize)]
//! # struct A(i32);
//! # #[ordr::producer]
//! # async fn a(_ctx: ordr::Context<()>) -> ordr::Result<A> { Ok(A(123)) }
//! # let job = ordr::Job::builder().add::<A>().build().unwrap();
//! let diagram = ordr::mermaid(&job);
//! println!("{diagram}");
//! ```
//!
//!
//! # Adding multiple targets to a job
//!
//! A job can have multiple targets. If two nodes depend on the same third node, it will only be
//! executed once.
//!
//! ```
//! # #[derive(Clone, serde::Serialize, serde::Deserialize)]
//! # struct A(i32);
//! # #[ordr::producer]
//! # async fn a(_ctx: ordr::Context<()>) -> ordr::Result<A> { Ok(A(123)) }
//! # #[derive(Clone, serde::Serialize, serde::Deserialize)]
//! # struct B(i32);
//! # #[ordr::producer]
//! # async fn b(_ctx: ordr::Context<()>) -> ordr::Result<B> { Ok(B(123)) }
//! let job = ordr::Job::builder()
//!     .add::<A>()
//!     .add::<B>()
//!     .build()
//!     .unwrap();
//! ```
//!
//!
//! # Partial data
//!
//! If you aleady have results from earlier, or maybe cached somewhere, then you
//! can add it to the job, and the graph will not run the producers for them (nor
//! its dependencies).
//!
//! ```
//! # async {
//! # #[derive(Clone, serde::Serialize, serde::Deserialize)]
//! # struct A(i32);
//! # #[ordr::producer]
//! # async fn a(_ctx: ordr::Context<()>) -> ordr::Result<A> { Ok(A(123)) }
//! let job = ordr::Job::builder().add::<A>().build().unwrap();
//! # let worker = ordr::Worker::new(job, ());
//!
//! // Worker from a previous job:
//! let data = worker.data().await;
//!
//! // Creating a new job with this data.
//! let job = ordr::Job::builder_with_data(data).add::<A>().build().unwrap();
//! # };
//! ```
//!
//!
//! # Stopping a job
//!
//! You can stop a job at any time. This can be useful for something like creating timeouts.
//!
//! ```
//! # async {
//! # let job = ordr::Job::builder().build().unwrap();
//! # let state = ();
//! let mut worker = ordr::Worker::new(job, state);
//!
//! // Starts the worker
//! worker.run();
//!
//! // Stops it and cancels all running nodes.
//! worker.stop();
//!
//! // You can still get the ouput.
//! let output = worker.get_output().await;
//!
//! // And whatever data was done before you stopped it.
//! let data = worker.data().await;
//! # };
//! ```

pub use ordr_core::*;
pub use ordr_macros::producer;
