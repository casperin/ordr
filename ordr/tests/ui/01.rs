use ordr::{build, producer, job::Job};

use std::convert::Infallible;

#[derive(Clone)]
struct Ctx;

#[derive(Clone, Debug)]
struct Raw(usize);
#[derive(Clone, Debug)]
struct Audio(usize);
#[derive(Clone, Debug)]
struct Video(usize);
#[derive(Clone, Debug)]
struct Mux(usize);

#[producer]
async fn raw(_ctx: Ctx) -> Result<Raw, Infallible> {
    Ok(Raw(1))
}

#[producer(output = Audio)]
async fn audio(_ctx: Ctx, raw: Raw) -> Result<Audio, Infallible> {
    Ok(Audio(2 + raw.0))
}

#[producer]
async fn video(_ctx: Ctx, raw: Raw) -> Result<Video, Infallible> {
    Ok(Video(4 + raw.0))
}

#[producer]
async fn mux(_ctx: Ctx, audio: Audio, video: Video) -> Result<Mux, Infallible> {
    Ok(Mux(8 + audio.0 + video.0))
}

#[tokio::main]
async fn main() {
    let graph = build!(Raw, Audio, Video, Mux).unwrap();
    let job = Job::new().with_target::<Mux>();

    let out = graph.execute(job, Ctx).await.unwrap();

    let mux = out.get::<Mux>().unwrap();
    assert_eq!(mux.0, 16);
}
