use std::time::Duration;

use ordr::{Context, Error, Job, Worker, producer};
use serde::{Deserialize, Serialize};

macro_rules! node {
    // No deps
    ( $name:ident: $ty:ident, $t:expr ) => {
        #[derive(Debug, Clone, Deserialize, Serialize)]
        struct $ty(u8);

        #[producer]
        async fn $name(_ctx: Context<()>) -> Result<$ty, Error> {
            tokio::time::sleep(std::time::Duration::from_millis($t)).await;
            Ok($ty(1))
        }
    };
    // Deps
    ( $name:ident: $ty:ident, $t:expr, $( $dep:ident ),* ) => {
        #[derive(Debug, Clone, Deserialize, Serialize)]
        struct $ty(u8);

        #[producer]
        async fn $name(_ctx: Context<()>, $( _: $dep ),* ) -> Result<$ty, Error> {
            tokio::time::sleep(std::time::Duration::from_millis($t)).await;
            Ok($ty(1))
        }
    };
}

node!(a: A, 10);
node!(b: B, 100, A);

#[tokio::test]
async fn can_stop() {
    let job = Job::builder().add::<B>().build().unwrap();
    let mut worker = Worker::new(job, ());
    worker.run().unwrap();
    tokio::time::sleep(Duration::from_millis(20)).await;
    worker.stop();
    let output = worker.get_output().await;
    assert!(output.is_stopped());
    let data = worker.data().await;
    assert!(data.contains_key("A"));
    assert!(!data.contains_key("B"));
}
