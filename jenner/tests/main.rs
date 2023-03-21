#![feature(generators, generator_trait, stmt_expr_attributes, async_iterator)]

use effective::EffectiveExt;
use jenner::generator;
use std::{
    async_iter::AsyncIterator,
    time::{Duration, Instant},
};

// #[tokio::test]
// async fn streams() {
//     let start = Instant::now();
//     let v = collect(double(countdown())).await;
//     assert_eq!(v, vec![10, 8, 6, 4, 2, 0]);
//     // expected to take around a second;
//     assert!(start.elapsed() > Duration::from_millis(200 * 5));
// }

#[generator]
#[yields]
async fn countdown() -> u32 {
    yield 5;

    for i in (0..5).rev() {
        tokio::time::sleep(Duration::from_millis(200)).await;
        yield i;
    }
}

// #[generator]
// #[yields]
// async fn double(input: impl AsyncIterator<Item = u32>) -> u32 {
//     for i in input {
//         yield i * 2;
//     }
//     .await;
// }

// #[generator]
// async fn collect<T: std::fmt::Debug>(input: impl AsyncIterator<Item = T>) -> Vec<T> {
//     let mut v = vec![];
//     for i in input {
//         println!("got {:?}", i);
//         v.push(i)
//     }
//     .await;
//     v
// }

#[test]
fn iterators() {
    let v: Vec<_> = fibonacii().shim().take(10).collect();
    assert_eq!(v, vec![0, 1, 1, 2, 3, 5, 8, 13, 21, 34]);
}

#[generator]
#[yields]
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

// #[tokio::test]
// async fn complete() {
//     let start = Instant::now();
//     let v = print(countdown1()).await;
//     assert_eq!(v, "done");
//     // expected to take around a second;
//     assert!(start.elapsed() > Duration::from_millis(200 * 5));
// }

// #[generator]
// async fn print(gen: impl AsyncGenerator<u32, &'static str>) -> &'static str {
//     for i in gen {
//         println!("got {:?}", i);
//     }
//     .await
//     .complete() // can be called since the loop has no breaks
// }

#[generator]
#[yields]
async fn countdown1() -> u32 {
    yield 5;
    for i in (0..5).rev() {
        tokio::time::sleep(Duration::from_millis(200)).await;
        yield i;
    }
}
