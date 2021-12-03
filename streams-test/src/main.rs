#![feature(generators)]

use std::time::Duration;

use futures_core::Stream;
use futures_util::{pin_mut, StreamExt};
use streams_generator::{stream};

#[tokio::main]
async fn main() {
    let s = double(zero_to_three());
    pin_mut!(s);

    while let Some(value) = s.next().await {
        println!("got {}", value);
    }
}

fn zero_to_three() -> impl Stream<Item = u32> {
    stream! {
        for i in 0..3 {
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

// // The above functions expands into the following:
//
// fn zero_to_three() -> impl Stream<Item = u32> {
//     unsafe {
//         ::streams_generator::new_stream(|mut __cx: ::streams_generator::UnsafeContextRef| {
//             for i in 0..3 {
//                 {
//                     let mut fut = { tokio::time::sleep(Duration::from_secs(1)) };
//                     loop {
//                         let polled = unsafe {
//                             ::std::future::Future::poll(
//                                 ::std::pin::Pin::new_unchecked(&mut fut),
//                                 __cx.get_context(),
//                             )
//                         };
//                         match polled {
//                             ::std::task::Poll::Ready(r) => break r,
//                             ::std::task::Poll::Pending => {
//                                 yield ::std::task::Poll::Pending;
//                             }
//                         }
//                     }
//                 };
//                 yield ::std::task::Poll::Ready({ i });
//             }
//         })
//     }
// }
//
// fn double(input: impl Stream<Item = u32>) -> impl Stream<Item = u32> {
//     unsafe {
//         ::streams_generator::new_stream(|mut __cx: ::streams_generator::UnsafeContextRef| {
//             let mut stream = input;
//             loop {
//                 let next = loop {
//                     let polled = unsafe {
//                         ::futures_core::stream::Stream::poll_next(
//                             ::std::pin::Pin::new_unchecked(&mut stream),
//                             __cx.get_context(),
//                         )
//                     };
//                     match polled {
//                         ::std::task::Poll::Ready(r) => break r,
//                         ::std::task::Poll::Pending => {
//                             yield ::std::task::Poll::Pending;
//                         }
//                     }
//                 };
//                 match next {
//                     Some(i) => {
//                         yield ::std::task::Poll::Ready({ i * 2 });
//                     }
//                     _ => break,
//                 }
//             }
//         })
//     }
// }
