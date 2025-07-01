use std::hint::black_box;
use std::time::Duration;

use ordr::{Output, build, error, executor, job::Job};

#[derive(Clone, Debug, PartialEq, Eq)]
struct Trees(usize);
#[derive(Clone, Debug, PartialEq, Eq)]
struct Friends(&'static [&'static str]);
#[derive(Clone, Debug, PartialEq, Eq)]
struct Money(usize);
#[derive(Clone, Debug, PartialEq, Eq)]
struct Paper(usize);
#[derive(Clone, Debug, PartialEq, Eq)]
struct Ideas(&'static [&'static str]);
#[derive(Clone, Debug, PartialEq, Eq)]
struct GetRich(usize);
#[derive(Clone, Debug, PartialEq, Eq)]
struct HaveFun(bool);

#[derive(Clone, Debug, Eq, PartialEq)]
struct Error(&'static str);

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[executor]
async fn trees(ctx: Ctx) -> Result<Trees, Error> {
    match ctx.trees {
        Some(v) => Ok(Trees(v)),
        None => Err(Error("trees err")),
    }
}

#[executor]
async fn friends(ctx: Ctx) -> Result<Friends, Error> {
    match ctx.friends {
        Some(v) => Ok(Friends(v)),
        None => Err(Error("friends err")),
    }
}

#[executor]
async fn money(ctx: Ctx) -> Result<Money, Error> {
    match ctx.money {
        Some(v) => Ok(Money(v)),
        None => Err(Error("money err")),
    }
}

#[executor]
async fn paper(ctx: Ctx, trees: Trees, friends: Friends) -> Result<Paper, Error> {
    black_box((trees, friends));
    let d = Duration::from_millis(ctx.chop);
    tokio::time::sleep(d).await;
    Ok(Paper(10000))
}

#[executor]
async fn ideas(ctx: Ctx, money: Money) -> Result<Ideas, Error> {
    black_box(money);
    let d = Duration::from_millis(ctx.travel);
    tokio::time::sleep(d).await;
    let ideas = &["Cats are great", "Dogs too"];

    if ctx.chop == 10101 {
        Err(Error("ideas err"))
    } else {
        Ok(Ideas(ideas))
    }
}

#[executor]
async fn get_rich(ctx: Ctx, paper: Paper, ideas: Ideas) -> Result<GetRich, Error> {
    black_box((paper, ideas));
    let d = Duration::from_millis(ctx.write);
    tokio::time::sleep(d).await;
    Ok(GetRich(100))
}

#[executor]
async fn have_fun(ctx: Ctx, friends: Friends, ideas: Ideas) -> Result<HaveFun, Error> {
    black_box((friends, ideas));
    let d = Duration::from_millis(ctx.play);
    tokio::time::sleep(d).await;
    Ok(HaveFun(true))
}

#[derive(Clone, Default)]
struct Ctx {
    trees: Option<usize>,
    friends: Option<&'static [&'static str]>,
    money: Option<usize>,
    chop: u64,
    travel: u64,
    write: u64,
    play: u64,
}

#[derive(Default, Output)]
struct Out {
    trees: Option<Trees>,
    friends: Option<Friends>,
    money: Option<Money>,
    paper: Option<Paper>,
    ideas: Option<Ideas>,
    get_rich: Option<GetRich>,
    have_fun: Option<HaveFun>,
}

#[tokio::main]
async fn main() {
    basic_flow().await;
    partial().await;
    timing_out().await;
    abort().await;
}

async fn basic_flow() {
    let graph = build!(Trees, Friends, Money, Paper, Ideas, GetRich, HaveFun).unwrap();

    let job = Job::new().with_target::<GetRich>().with_target::<HaveFun>();

    let ctx = Ctx {
        trees: Some(10),
        friends: Some(&["Poul", "Bob", "Ida"]),
        money: Some(4),
        ..Ctx::default()
    };

    let output = graph.execute(job, ctx).await.unwrap();
    let out = Out::default().with_output_from(&output);

    assert_eq!(out.paper, Some(Paper(10000)));
    assert_eq!(out.have_fun, Some(HaveFun(true)));
}

async fn partial() {
    let graph = build!(Trees, Friends, Money, Paper, Ideas, GetRich, HaveFun).unwrap();

    let job = Job::new()
        .with_input(Ideas(&["ida"]))
        .with_target::<HaveFun>();

    let ctx = Ctx {
        friends: Some(&["idea"]),
        ..Ctx::default()
    };

    let output = graph.execute(job, ctx).await.unwrap();
    let out = Out::default().with_output_from(&output);

    assert_eq!(out.friends, Some(Friends(&["idea"])));
    assert_eq!(out.have_fun, Some(HaveFun(true)));
    assert_eq!(out.paper, None);
    assert_eq!(out.get_rich, None);
}

async fn timing_out() {
    let graph = build!(Trees, Friends, Money, Paper, Ideas, GetRich, HaveFun).unwrap();
    let job = Job::new().with_target::<GetRich>().with_target::<HaveFun>();

    let ctx = Ctx {
        trees: Some(10),
        friends: Some(&["Poul", "Bob", "Ida"]),
        money: Some(4),
        chop: 1,
        travel: 10,
        write: 10,
        play: 10,
    };

    let token = job.cancellation_token();

    tokio::spawn(async move {
        tokio::time::sleep(Duration::from_millis(5)).await;
        token.cancel();
    });

    let error = graph.execute(job, ctx).await.unwrap_err();

    let error::Error::Cancelled { outputs, .. } = error else {
        panic!("Expected cancelled, got {error:?}");
    };

    let out = Out::default().with_output_from(&outputs);

    assert!(out.friends.is_some());
    assert!(out.money.is_some());
    assert!(out.paper.is_some());
    assert!(out.trees.is_some());

    // We assert this one specifically, because re-running would change its value.
    assert_eq!(out.trees, Some(Trees(10)));

    let job2 = out
        .into_job()
        .with_target::<GetRich>()
        .with_target::<HaveFun>();

    let ctx2 = Ctx {
        trees: Some(20),
        friends: Some(&["Amigo"]),
        money: Some(8),
        ..Ctx::default()
    };

    let outputs = graph.execute(job2, ctx2).await.unwrap();
    let out = Out::default().with_output_from(&outputs);

    assert!(out.get_rich.is_some());
    assert!(out.have_fun.is_some());
    assert!(out.ideas.is_some());
    // Make sure this didn't change, despite the value we set in Ctx.
    assert_eq!(out.trees, Some(Trees(10)));
}

async fn abort() {
    let graph = build!(Trees, Friends, Money, Paper, Ideas, GetRich, HaveFun).unwrap();

    let job = Job::new().with_target::<GetRich>().with_target::<HaveFun>();

    let ctx = Ctx {
        trees: Some(10),
        friends: Some(&["Poul", "Bob", "Ida"]),
        money: Some(4),
        chop: 10101, // Makes `ideas` fail
        travel: 10,
        write: 10,
        play: 10,
    };

    let error = graph.execute(job, ctx).await.unwrap_err();

    let error::Error::NodeFailed {
        i, error, outputs, ..
    } = error
    else {
        panic!("Expected cancelled, got {error:?}");
    };

    let out = Out::default().with_output_from(&outputs);

    assert_eq!(graph.node_name(i), "Ideas");
    assert_eq!(error, Error("ideas err"));

    assert!(out.friends.is_some());
    assert!(out.money.is_some());
    assert!(out.trees.is_some());
}
