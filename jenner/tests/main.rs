#![feature(coroutines)]

use jenner::{effect, AsyncStream, SyncGeneratorExt};
use std::{
    pin::pin,
    time::{Duration, Instant},
};

#[tokio::test]
async fn streams() {
    let start = Instant::now();
    let v = collect(double(countdown())).await;
    assert_eq!(v, vec![10, 8, 6, 4, 2, 0]);
    // expected to take around a second;
    assert!(start.elapsed() > Duration::from_millis(200 * 5));
}

#[effect(yields)]
async fn countdown() -> u32 {
    yield 5;

    for i in (0..5).rev() {
        tokio::time::sleep(Duration::from_millis(200)).await;
        yield i;
    }
}

#[effect(yields)]
async fn double(input: impl AsyncStream<u32>) -> u32 {
    #[effect(async)]
    for i in input {
        yield i * 2;
    }
}

#[effect]
async fn collect<T: std::fmt::Debug>(input: impl AsyncStream<T>) -> Vec<T> {
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
    let mut generator = pin!(fibonacii());
    let v: Vec<_> = generator.as_mut().iter().take(10).collect();
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
    print(countdown1()).await;
    // expected to take around a second;
    assert!(start.elapsed() > Duration::from_millis(200 * 5));
}

#[effect]
async fn print(input: impl AsyncStream<u32>) {
    #[effect(async)]
    for i in input {
        println!("got {:?}", i);
    }
}

#[effect(yields)]
async fn countdown1() -> u32 {
    yield 5;
    for i in (0..5).rev() {
        tokio::time::sleep(Duration::from_millis(200)).await;
        yield i;
    }
}
