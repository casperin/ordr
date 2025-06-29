use ordr::{build_graph, executor, job::Job};

use std::convert::Infallible;

#[derive(Clone, Debug)]
struct Ctx;

#[derive(Clone, Debug)]
struct Raw(usize);
#[derive(Clone, Debug)]
struct Audio(usize);
#[allow(unused)]
#[derive(Clone, Debug)]
struct Mux(usize);

#[executor]
async fn raw(_ctx: Ctx) -> Result<Raw, Infallible> {
    Ok(Raw(1))
}

#[executor]
async fn audio(_ctx: Ctx, raw: Raw) -> Result<Audio, Infallible> {
    Ok(Audio(2 + raw.0))
}

#[executor(name="Muxies", output = Mux)]
async fn mux(_ctx: Ctx, audio: Audio, video: foo::Video) -> Result<Mux, Infallible> {
    Ok(Mux(8 + audio.0 + video.0))
}

mod foo {
    #[derive(Clone, Debug)]
    pub struct Video(pub usize);

    #[super::executor]
    pub async fn video(_ctx: super::Ctx, raw: super::Raw) -> Result<Video, super::Infallible> {
        Ok(Video(4 + raw.0))
    }
}

#[tokio::main]
pub async fn main() {
    let graph = build_graph!(Raw, Audio, foo::Video, Mux).unwrap();

    // Add stuff we actually already have
    let job = Job::new()
        .with_input(Audio(3))
        .with_input(foo::Video(3))
        .with_target::<Mux>();

    graph.validate_job(&job).unwrap();
}
