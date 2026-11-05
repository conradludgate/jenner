#![feature(coroutines)]

use jenner::{
    effect,
    effective::{EffectResult, Effective},
};
use std::{
    pin::pin,
    task::{Context, Waker},
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
        pin!(sync_value(7)).poll_effect(&mut cx),
        EffectResult::Item(7)
    ));
    assert!(matches!(
        pin!(async_value(9)).poll_effect(&mut cx),
        EffectResult::Item(9)
    ));
}

#[test]
fn fallible_single_values_report_success_and_failure() {
    let mut cx = Context::from_waker(Waker::noop());
    assert!(matches!(
        pin!(sync_fallible(Ok(7))).poll_effect(&mut cx),
        EffectResult::Item(7)
    ));
    assert!(matches!(
        pin!(async_fallible(Ok(9))).poll_effect(&mut cx),
        EffectResult::Item(9)
    ));
    assert!(
        matches!(pin!(sync_fallible(Err("bad"))).poll_effect(&mut cx), EffectResult::Failure(e) if e.0 == "bad")
    );
    assert!(
        matches!(pin!(async_fallible(Err("bad"))).poll_effect(&mut cx), EffectResult::Failure(e) if e.0 == "bad")
    );
}

#[test]
fn fallible_generators_yield_before_reporting_failure() {
    let mut cx = Context::from_waker(Waker::noop());
    let mut sync = pin!(sync_fallible_items());
    assert!(matches!(
        sync.as_mut().poll_effect(&mut cx),
        EffectResult::Item(1)
    ));
    assert!(
        matches!(sync.as_mut().poll_effect(&mut cx), EffectResult::Failure(e) if e.0 == "stop")
    );
    let mut asynchronous = pin!(async_fallible_items());
    assert!(matches!(
        asynchronous.as_mut().poll_effect(&mut cx),
        EffectResult::Item(1)
    ));
    assert!(
        matches!(asynchronous.as_mut().poll_effect(&mut cx), EffectResult::Failure(e) if e.0 == "stop")
    );
}
