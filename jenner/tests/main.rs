#![feature(generators)]

use effective::{wrappers::future, Async, Effective, EffectiveExt, Multiple};
use jenner::effect;
use std::{
    convert::Infallible,
    pin::pin,
    time::{Duration, Instant},
};

#[tokio::test]
async fn streams() {
    let start = Instant::now();
    let v = collect(double(countdown())).shim().await;
    assert_eq!(v, vec![10, 8, 6, 4, 2, 0]);
    // expected to take around a second;
    assert!(start.elapsed() > Duration::from_millis(200 * 5));
}

#[effect(yields)]
async fn countdown() -> u32 {
    yield 5;

    for i in (0..5).rev() {
        future(tokio::time::sleep(Duration::from_millis(200))).await;
        yield i;
    }
}

#[effect(yields)]
async fn double(
    input: impl Effective<Item = u32, Failure = Infallible, Produces = Multiple, Async = Async>,
) -> u32 {
    #[effect(async)]
    for i in input {
        yield i * 2;
    }
}

#[effect]
async fn collect<T: std::fmt::Debug>(
    input: impl Effective<Item = T, Failure = Infallible, Produces = Multiple, Async = Async>,
) -> Vec<T> {
    let mut v = vec![];
    #[effect(async)]
    for i in input {
        println!("got {:?}", i);
        v.push(i)
    }
    v
}

#[test]
fn iterators() {
    let v: Vec<_> = pin!(fibonacii()).shim().take(10).collect();
    assert_eq!(v, vec![0, 1, 1, 2, 3, 5, 8, 13, 21, 34]);
}

#[effect(yields)]
fn fibonacii() -> usize {
    let mut a = 0;
    let mut b = 1;
    loop {
        let tmp = a;
        a = b;
        b += tmp;
        yield tmp;
    }
}

#[tokio::test]
async fn complete() {
    let start = Instant::now();
    print(countdown1()).shim().await;
    // expected to take around a second;
    assert!(start.elapsed() > Duration::from_millis(200 * 5));
}

#[effect]
async fn print(
    input: impl Effective<Item = u32, Failure = Infallible, Produces = Multiple, Async = Async>,
) {
    #[effect(async)]
    for i in input {
        println!("got {:?}", i);
    }
}

#[effect(yields)]
async fn countdown1() -> u32 {
    yield 5;
    for i in (0..5).rev() {
        future(tokio::time::sleep(Duration::from_millis(200))).await;
        yield i;
    }
}
