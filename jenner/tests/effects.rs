#![feature(coroutines, coroutine_trait)]

use jenner::{effect, AsyncGenerator, Coroutine, CoroutineState, Stream, SyncGeneratorExt};
use std::{
    future::Future,
    pin::pin,
    task::{Context, Poll, Waker},
};

#[effect]
fn sync_value(value: u32) -> u32 {
    value
}
#[effect]
async fn async_value(value: u32) -> u32 {
    value
}
#[effect(fallible)]
fn sync_fallible(value: Result<u32, &'static str>) -> Result<u32, &'static str> {
    value?
}
#[effect(fallible)]
async fn async_fallible(value: Result<u32, &'static str>) -> Result<u32, &'static str> {
    value?
}
#[effect(yields, fallible)]
fn sync_fallible_items() -> Result<u32, &'static str> {
    yield 1;
    Err("stop")?;
}
#[effect(yields, fallible)]
async fn async_fallible_items() -> Result<u32, &'static str> {
    yield 1;
    Err("stop")?;
}

#[test]
fn single_values_capture_copy_arguments() {
    let mut cx = Context::from_waker(Waker::noop());
    assert!(matches!(
        pin!(sync_value(7)).resume(()),
        CoroutineState::Complete(7)
    ));
    assert_eq!(pin!(async_value(9)).poll(&mut cx), Poll::Ready(9));
}

#[test]
fn fallible_single_values_report_success_and_failure() {
    let mut cx = Context::from_waker(Waker::noop());
    assert!(matches!(
        pin!(sync_fallible(Ok(7))).resume(()),
        CoroutineState::Complete(Ok(7))
    ));
    assert_eq!(
        pin!(async_fallible(Ok(9))).poll(&mut cx),
        Poll::Ready(Ok(9))
    );
    assert!(matches!(
        pin!(sync_fallible(Err("bad"))).resume(()),
        CoroutineState::Complete(Err("bad"))
    ));
    assert_eq!(
        pin!(async_fallible(Err("bad"))).poll(&mut cx),
        Poll::Ready(Err("bad"))
    );
}

#[test]
fn fallible_generators_keep_errors_separate_from_items() {
    let mut sync = pin!(sync_fallible_items());
    assert!(matches!(
        sync.as_mut().resume(()),
        CoroutineState::Yielded(1)
    ));
    assert!(matches!(
        sync.as_mut().resume(()),
        CoroutineState::Complete(Err("stop"))
    ));
    let mut cx = Context::from_waker(Waker::noop());
    let mut asynchronous = pin!(async_fallible_items());
    assert!(matches!(
        asynchronous.as_mut().poll_resume(&mut cx),
        Poll::Ready(CoroutineState::Yielded(1))
    ));
    assert!(matches!(
        asynchronous.as_mut().poll_resume(&mut cx),
        Poll::Ready(CoroutineState::Complete(Err("stop")))
    ));
}

#[test]
fn stream_and_iterator_adapters_emit_terminal_errors_once() {
    let mut sync = pin!(sync_fallible_items());
    let mut iter = sync.as_mut().try_iter();
    assert_eq!(iter.next(), Some(Ok(1)));
    assert_eq!(iter.next(), Some(Err("stop")));
    assert_eq!(iter.next(), None);
    assert_eq!(iter.next(), None);
    let mut cx = Context::from_waker(Waker::noop());
    let mut asynchronous = pin!(async_fallible_items());
    assert_eq!(
        asynchronous.as_mut().poll_next(&mut cx),
        Poll::Ready(Some(Ok(1)))
    );
    assert_eq!(
        asynchronous.as_mut().poll_next(&mut cx),
        Poll::Ready(Some(Err("stop")))
    );
    assert_eq!(asynchronous.as_mut().poll_next(&mut cx), Poll::Ready(None));
    assert_eq!(asynchronous.as_mut().poll_next(&mut cx), Poll::Ready(None));
}

#[effect(fallible)]
fn eager_sync_value(calls: std::rc::Rc<std::cell::Cell<usize>>) -> Result<u32, &'static str> {
    calls.set(calls.get() + 1);
    7
}

#[test]
fn plain_synchronous_fallible_bodies_execute_at_construction() {
    let calls = std::rc::Rc::new(std::cell::Cell::new(0));
    let value = eager_sync_value(calls.clone());
    assert_eq!(calls.get(), 1);
    assert!(matches!(
        pin!(value).resume(()),
        CoroutineState::Complete(Ok(7))
    ));
    assert_eq!(calls.get(), 1);
}
