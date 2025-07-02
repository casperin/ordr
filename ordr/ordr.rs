#![warn(missing_docs)]

//! Ordr (タコ) is a library that helps you execute and keep track of a set of interdependent functions.
//!
//! It can create a graph (specifically a `DAG`) of functions depending on functions, and execute them as they get ready, in parallel.
//!
//! Here is a simple example taken from one of the examples ([chatty](./examples/chatty.rs)) in the repo:
//!
//! ```ignore,txt
//! ╭─────> C ────╮
//! A             ├─> E
//! ╰──> B ──> D ─╯
//! ```
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
//! output of an "producer"; an `async` function that takes a "context", any number of other nodes,
//! and returns a `Result<A, YourError>`. The context, is anything you want as long as it implements
//! `Clone`. It's meant to be used for having database connections or whatever else you need.
//!
//! It looks like this:
//!
//! ```ignore
//! #[derive(Clone)]
//! struct Ctx {
//!     // Whatever we need
//! }
//!
//! // Our node `A`.
//! #[derive(Clone)]
//! struct A(i32);
//!
//! #[producer]
//! async fn my_a_producer(ctx: Ctx) -> Result<A, Infallible> {
//!     // Do some actual work
//!     Ok(A(123))
//! }
//! ```
//!
//! If we then have a node `B` that depends on `A`, we just add it to the arguments:
//!
//! ```ignore
//! #derive(Clone)
//! struct B(i32);
//!
//! #[producer]
//! async fn make_b(ctx: Ctx, a: A) -> Result<B, Infallible> {
//!     Ok(B(a.0 + 2))
//! }
//! ```
//!
//! Before we start executing anything, we need to make a graph:
//!
//! ```ignore
//! // Checks for cycles, etc. You can reuse the graph (although it's not terribly
//! // expensive to make one).
//! let graph = build!(A, B).unwrap();
//!
//! // Then we need a job. A job describes the targets we are interested in. You
//! // can add as many targets as you like.
//! let job = Job::new().with_target::<B>();
//!
//! // We also need the Context. If your tasks don't need a context, just use `()`.
//! let ctx = Ctx {};
//!
//! // And we are ready to execute the job. This will execute my_a_producer(ctx)
//! // and then make_b(ctx, result_of_a).
//! let outputs = graph.execute(job, ctx).await.unwrap();
//!
//! let res_a = outputs.get::<A>();
//! assert_eq!(res_a, Some(&A(123)));
//!
//! let res_b = outputs.get::<B>();
//! assert_eq!(res_b, Some(&B(125)));
//! ```
//!
//! A few things to keep in mind:
//!
//! * All nodes and the context must implement `Clone`.
//! * This is required since both `B` and `C` requires `A` (and `Ctx`).
//! * All producers must return the same type of error.
//! * All producers must be async and take the context as first parameter.
//!
//! ## Working with outputs
//!
//! It's a bit cumbersome to get the outputs out in the example above, but you can
//! do this instead.
//!
//! ```ignore
//! #[derive(Default, Output)] // <-- custom derive
//! struct MyResults {
//!     a: Option<A>, // <-- all must be Option<Node>
//!     b: Option<B>,
//! }
//!
//! // Execute some job
//! let outputs = graph.execute(job, ctx).await.unwrap();
//!
//! // Clones the results from `outputs` into `my_results`.
//! let my_results = MyResults::default().with_output_from(&outputs);
//!
//! // You can then serialize it or whatever you need to do. You can also turn it
//! // into another job that will continue where this one left off.
//! //
//! // Since we got this one from the success case, this job is useless, but you
//! // also get an `outputs` in case of an error (or cancellation), that may or may
//! // not have finished fully.
//! let job2 = my_results.into_job().with_target::<A>();
//! ```
//!
//! ## Mermaid diagram
//!
//! It might be useful to inspect the graph and how ordr is expecting to
//! execute a job. You can get a graph like this:
//!
//! ```ignore
//! let graph = build!(A, B);
//! let job = Job::new().with_target::<B>();
//! let mermaid = graph.mermaid(&job);
//! println!("{mermaid}");
//! ```
//!
//! ## Adding multiple targets to a job
//!
//! A target is what the graph will solve for. It will only do as much work as is needed, to get to a
//! point where all targets have been run.
//!
//! ```ignore
//! let job = Job::new().with_target::<A>().with_target::<B>();
//!
//! // or
//! let mut job = Job::new();
//! job.target::<A>(); // The funny syntax is because `A` is a type, not the concrete struct.
//! job.target::<B>();
//! ```
//!
//! ## Adding input to a job
//!
//! If you aleady have results from earlier, or maybe cached somewhere, then you
//! can add it to the job, and the graph will not run the producers for them (nor
//! its dependencies).
//!
//! ```ignore
//! let mut job = Job::new();
//! job.input(A(22)); // You can add as many as you like
//! ```
//!
//! ## Cancelling a job
//!
//! You can get a cancellation token out of a job, that you can then later cancel.
//!
//! This is useful for something like timeouts.
//!
//! ```ignore
//! let job = Job::new(); // remember to add a target
//! let cancellation_token = job.cancellation_token();
//!
//! // Later during execution...
//! cancellation_token.cancel();
//! ```

pub use ordr_core::{build, error, graph, job, node, outputs};
pub use ordr_macros::{Output, producer};
