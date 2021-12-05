#![feature(generators)]
#![feature(stmt_expr_attributes)]

use futures_core::Stream;
use jenner::generator;
use std::time::{Duration, Instant};

#[tokio::test]
async fn streams() {
    let start = Instant::now();
    let v = collect(double(countdown())).await;
    assert_eq!(v, vec![10, 8, 6, 4, 2, 0]);
    // expected to take around a second;
    assert!(start.elapsed() > Duration::from_millis(200 * 5));
}

#[generator]
#[yields(u32)]
async fn countdown() {
    yield 5;
    for i in (0..5).rev() {
        tokio::time::sleep(Duration::from_millis(200)).await;
        yield i;
    }
}

#[generator]
#[yields(u32)]
async fn double(input: impl Stream<Item = u32>) {
    for i in input {
        yield i * 2;
    }.await;
}

#[generator]
async fn collect<T: std::fmt::Debug>(input: impl Stream<Item = T>) -> Vec<T> {
    let mut v = vec![];
    for i in input {
        println!("got {:?}", i);
        v.push(i)
    }.await;
    v
}

#[test]
fn iterators() {
    let v: Vec<_> = fibonacii().take(10).collect();
    assert_eq!(v, vec![0, 1, 1, 2, 3, 5, 8, 13, 21, 34]);
}

#[generator]
#[yields(usize)]
fn fibonacii() {
    let mut a = 0;
    let mut b = 1;
    loop {
        let tmp = a;
        a = b;
        b += tmp;
        yield tmp;
    }
}
