#![feature(generators)]

use std::future::Future;
use std::time::Duration;

use futures_core::Stream;
use streams_generator::stream;

#[tokio::main]
async fn main() {
    let v = collect(double(zero_to_three())).await;
    assert_eq!(v, vec![8, 6, 4, 2, 0]);
}

fn zero_to_three() -> impl Stream<Item = u32> {
    stream! {
        for i in (0..5).rev() {
            tokio::time::sleep(Duration::from_secs(1)).await;
            yield i;
        }
    }
}

fn double(input: impl Stream<Item = u32>) -> impl Stream<Item = u32> {
    stream! {
        async for i in input {
            yield i * 2;
        }
    }
}

fn collect<T: std::fmt::Debug>(input: impl Stream<Item = T>) -> impl Future<Output = Vec<T>> {
    stream! {
        let mut v = vec![];
        async for i in input {
            println!("got {:?}", i);
            v.push(i)
        }
        v
    }
}

// // The above functions expands into the following:
//
// fn zero_to_three() -> impl Stream<Item = u32> {
//     unsafe {
//         ::streams_generator::new_stream_generator::<_, _, _>(
//             |mut __cx: ::streams_generator::UnsafeContextRef| {
//                 for i in (0..5).rev() {
//                     {
//                         let mut fut = { tokio::time::sleep(Duration::from_secs(1)) };
//                         loop {
//                             let polled = unsafe {
//                                 ::std::future::Future::poll(
//                                     ::std::pin::Pin::new_unchecked(&mut fut),
//                                     __cx.get_context(),
//                                 )
//                             };
//                             match polled {
//                                 ::std::task::Poll::Ready(r) => break r,
//                                 ::std::task::Poll::Pending => {
//                                     yield ::std::task::Poll::Pending;
//                                 }
//                             }
//                         }
//                     };
//                     yield ::std::task::Poll::Ready({ i });
//                 }
//             },
//         )
//     }
// }
//
// fn double(input: impl Stream<Item = u32>) -> impl Stream<Item = u32> {
//     unsafe {
//         ::streams_generator::new_stream_generator::<_, _, _>(
//             |mut __cx: ::streams_generator::UnsafeContextRef| {
//                 let mut stream = input;
//                 loop {
//                     let next = loop {
//                         let polled = unsafe {
//                             ::futures_core::stream::Stream::poll_next(
//                                 ::std::pin::Pin::new_unchecked(&mut stream),
//                                 __cx.get_context(),
//                             )
//                         };
//                         match polled {
//                             ::std::task::Poll::Ready(r) => break r,
//                             ::std::task::Poll::Pending => {
//                                 yield ::std::task::Poll::Pending;
//                             }
//                         }
//                     };
//                     match next {
//                         Some(i) => {
//                             yield ::std::task::Poll::Ready({ i * 2 });
//                         }
//                         _ => break,
//                     };
//                 }
//             },
//         )
//     }
// }
//
// fn collect<T: std::fmt::Debug>(input: impl Stream<Item = T>) -> impl Future<Output = Vec<T>> {
//     unsafe {
//         ::streams_generator::new_stream_generator::<(), _, _>(
//             |mut __cx: ::streams_generator::UnsafeContextRef| {
//                 let mut v = ::alloc::vec::Vec::new();
//                 {
//                     let mut stream = input;
//                     loop {
//                         let next = loop {
//                             let polled = unsafe {
//                                 ::futures_core::stream::Stream::poll_next(
//                                     ::std::pin::Pin::new_unchecked(&mut stream),
//                                     __cx.get_context(),
//                                 )
//                             };
//                             match polled {
//                                 ::std::task::Poll::Ready(r) => break r,
//                                 ::std::task::Poll::Pending => {
//                                     yield ::std::task::Poll::Pending;
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
