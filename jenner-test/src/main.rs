#![feature(generators)]
#![feature(stmt_expr_attributes)]

use std::time::Duration;
use jenner::{exports::Stream, generator};

#[tokio::main]
async fn main() {
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

// // The above functions expands into the following:
//
// fn countdown() -> impl ::jenner::StreamGenerator<(u32), ()> {
//     unsafe {
//         ::jenner::new_stream_generator(
//             |mut __cx: ::jenner::UnsafeContextRef| {
//                 for i in (0..5).rev() {
//                     {
//                         let mut fut = { tokio::time::sleep(Duration::from_secs(1)) };
//                         loop {
//                             let polled = unsafe {
//                                 ::jenner::exports::Future::poll(
//                                     ::jenner::exports::pin::Pin::new_unchecked(&mut fut),
//                                     __cx.get_context(),
//                                 )
//                             };
//                             match polled {
//                                 ::jenner::exports::task::Poll::Ready(r) => break r,
//                                 ::jenner::exports::task::Poll::Pending => {
//                                     yield ::jenner::exports::task::Poll::Pending;
//                                 }
//                             }
//                         }
//                     };
//                     yield ::jenner::exports::task::Poll::Ready({ i });
//                 }
//             },
//         )
//     }
// }
//
// fn double(input: impl Stream<Item = u32>) -> impl ::jenner::StreamGenerator<(u32), ()> {
//     unsafe {
//         ::jenner::new_stream_generator(
//             |mut __cx: ::jenner::UnsafeContextRef| {
//                 let mut stream = input;
//                 loop {
//                     let next = loop {
//                         let polled = unsafe {
//                             ::jenner::exports::Stream::poll_next(
//                                 ::jenner::exports::pin::Pin::new_unchecked(&mut stream),
//                                 __cx.get_context(),
//                             )
//                         };
//                         match polled {
//                             ::jenner::exports::task::Poll::Ready(r) => break r,
//                             ::jenner::exports::task::Poll::Pending => {
//                                 yield ::jenner::exports::task::Poll::Pending;
//                             }
//                         }
//                     };
//                     match next {
//                         Some(i) => {
//                             yield ::jenner::exports::task::Poll::Ready({ i * 2 });
//                         }
//                         _ => break,
//                     };
//                 }
//             },
//         )
//     }
// }
//
// fn collect<T: std::fmt::Debug>(input: impl Stream<Item = T>) -> impl ::jenner::StreamGenerator<(), Vec<T>> {
//     unsafe {
//         ::jenner::new_stream_generator(
//             |mut __cx: ::jenner::UnsafeContextRef| {
//                 let mut v = ::alloc::vec::Vec::new();
//                 {
//                     let mut stream = input;
//                     loop {
//                         let next = loop {
//                             let polled = unsafe {
//                                 ::jenner::exports::Stream::poll_next(
//                                     ::jenner::exports::pin::Pin::new_unchecked(&mut stream),
//                                     __cx.get_context(),
//                                 )
//                             };
//                             match polled {
//                                 ::jenner::exports::task::Poll::Ready(r) => break r,
//                                 ::jenner::exports::task::Poll::Pending => {
//                                     yield ::jenner::exports::task::Poll::Pending;
//                                 }
//                             }
//                         };
//                         match next {
//                             Some(i) => {
//                                 println!("got {:?}", i);
//                                 v.push(i)
//                             }
//                             _ => break,
//                         };
//                     }
//                 }
//                 v
//             },
//         )
//     }
// }
