#![feature(coroutines, coroutine_trait)]

use futures_util::{stream, StreamExt};
use jenner::{
    effect, AsyncGenerator, AsyncStream, AsyncValue, Coroutine, CoroutineState, Stream,
    SyncGeneratorExt, TryAsyncStream,
};
use std::{
    future::Future,
    pin::pin,
    task::{Context, Poll, Waker},
};

fn run_ready<F: Future>(future: F) -> F::Output {
    match pin!(future).poll(&mut Context::from_waker(Waker::noop())) {
        Poll::Ready(output) => output,
        Poll::Pending => panic!("test future unexpectedly pending"),
    }
}

#[effect(yields(u32))]
async fn final_value() -> String {
    let label = String::from("finished");
    let borrowed = &label;
    yield std::future::ready(10).await;
    yield 20;
    borrowed.to_owned()
}

#[effect(yields(u32))]
fn sync_final_value() -> String {
    yield 10;
    String::from("finished")
}

#[test]
fn yielded_items_and_final_values_have_separate_types() {
    let mut cx = Context::from_waker(Waker::noop());
    let mut generator = pin!(final_value());
    assert!(matches!(
        generator.as_mut().poll_resume(&mut cx),
        Poll::Ready(CoroutineState::Yielded(10))
    ));
    assert!(matches!(
        generator.as_mut().poll_resume(&mut cx),
        Poll::Ready(CoroutineState::Yielded(20))
    ));
    assert!(
        matches!(generator.as_mut().poll_resume(&mut cx), Poll::Ready(CoroutineState::Complete(s)) if s == "finished")
    );
    let mut generator = pin!(sync_final_value());
    assert!(matches!(
        generator.as_mut().resume(()),
        CoroutineState::Yielded(10)
    ));
    assert!(
        matches!(generator.as_mut().resume(()), CoroutineState::Complete(s) if s == "finished")
    );
}

#[effect]
async fn nested() -> Result<u32, String> {
    async fn local() -> u32 {
        std::future::ready(2).await
    }
    let closure = || async { std::future::ready(3).await };
    let value = std::future::ready(async { std::future::ready(Ok::<_, &str>(7)).await })
        .await
        .await?;
    Ok(value + closure().await + local().await)
}

#[test]
fn nested_awaits_try_and_suspension_boundaries_work_with_native_futures() {
    assert_eq!(run_ready(nested()), Ok(12));
}

#[effect(yields)]
async fn values() -> u32 {
    yield 1;
    yield 2;
}

#[effect]
async fn value() -> u32 {
    7
}

type Fallible<T> = Result<T, String>;

#[effect(yields, fallible)]
async fn fallible_values(fail: bool) -> Fallible<u32> {
    yield 1;
    if fail {
        std::future::ready(Err::<(), &str>("stop")).await?;
    }
    yield 2;
}

#[effect(yields, fallible)]
fn sync_fallible_values() -> Fallible<u32> {
    yield 1;
    yield 2;
}

async fn await_alias(value: impl AsyncValue<u32>) -> u32 {
    value.await
}
async fn collect_alias(input: impl AsyncStream<u32>) -> Vec<u32> {
    input.collect().await
}
async fn collect_try_alias(input: impl TryAsyncStream<u32, String>) -> Vec<Result<u32, String>> {
    input.collect().await
}

#[test]
fn alias_bounds_expose_native_future_and_stream_operations() {
    assert_eq!(run_ready(await_alias(value())), 7);
    assert_eq!(run_ready(collect_alias(values())), vec![1, 2]);
    assert_eq!(
        run_ready(collect_try_alias(fallible_values(true))),
        vec![Ok(1), Err(String::from("stop"))]
    );
}

#[test]
fn fallible_adapters_finish_successfully_without_an_extra_item() {
    assert_eq!(
        run_ready(collect_try_alias(fallible_values(false))),
        vec![Ok(1), Ok(2)]
    );
    let mut generator = pin!(sync_fallible_values());
    let mut iter = generator.as_mut().try_iter();
    assert_eq!(iter.next(), Some(Ok(1)));
    assert_eq!(iter.next(), Some(Ok(2)));
    assert_eq!(iter.next(), None);
    assert_eq!(iter.next(), None);
}

#[effect(fallible)]
async fn sum(input: impl Stream<Item = Result<u32, String>>) -> Result<u32, String> {
    let mut total = 0;
    #[effect(async)]
    for item in input {
        total += item?;
    }
    total
}

#[test]
fn async_for_accepts_external_streams_and_propagates_errors() {
    assert_eq!(run_ready(sum(stream::iter([Ok(1), Ok(2)]))), Ok(3));
    assert_eq!(
        run_ready(sum(stream::iter([Ok(1), Err(String::from("stop")), Ok(2)]))),
        Err(String::from("stop"))
    );
}

#[test]
fn pinned_trait_objects_forward_generator_polling() {
    let mut cx = Context::from_waker(Waker::noop());
    let mut generator: std::pin::Pin<Box<dyn AsyncGenerator<Yield = u32, Return = String>>> =
        Box::pin(final_value());
    // Exercise AsyncGenerator's Pin<P> implementation with both Box and &mut.
    let mut pinned = pin!(generator.as_mut());
    assert!(matches!(
        pinned.as_mut().poll_resume(&mut cx),
        Poll::Ready(CoroutineState::Yielded(10))
    ));
    assert!(matches!(
        pinned.as_mut().poll_resume(&mut cx),
        Poll::Ready(CoroutineState::Yielded(20))
    ));
    assert!(
        matches!(pinned.as_mut().poll_resume(&mut cx), Poll::Ready(CoroutineState::Complete(s)) if s == "finished")
    );
}

#[effect(yields(u32))]
async fn terminal_error() -> Result<(), &'static str> {
    yield 1;
    Err("stop")
}

#[test]
fn explicit_yield_types_preserve_a_result_return_without_wrapping() {
    let mut cx = Context::from_waker(Waker::noop());
    let mut generator = pin!(terminal_error());
    assert!(matches!(
        generator.as_mut().poll_resume(&mut cx),
        Poll::Ready(CoroutineState::Yielded(1))
    ));
    assert!(matches!(
        generator.as_mut().poll_resume(&mut cx),
        Poll::Ready(CoroutineState::Complete(Err("stop")))
    ));
}
