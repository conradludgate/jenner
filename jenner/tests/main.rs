#![feature(generators)]
#![feature(stmt_expr_attributes)]

use jenner::{exports::Stream, generator};
use std::time::Duration;

#[tokio::test]
async fn streams() {
    let v = collect(double(countdown())).await;
    assert_eq!(v, vec![8, 6, 4, 2, 0]);
}

#[generator]
#[yields(u32)]
async fn countdown() {
    for i in (0..5).rev() {
        tokio::time::sleep(Duration::from_secs(1)).await;
        yield i;
    }
}

#[generator]
#[yields(u32)]
async fn double(input: impl Stream<Item = u32>) {
    #[async_for]
    for i in input {
        yield i * 2;
    }
}

#[generator]
async fn collect<T: std::fmt::Debug>(input: impl Stream<Item = T>) -> Vec<T> {
    let mut v = vec![];
    #[async_for]
    for i in input {
        println!("got {:?}", i);
        v.push(i)
    }
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
