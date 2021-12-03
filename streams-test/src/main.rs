#![feature(generators)]

use std::time::Duration;

use futures_core::Stream;
use futures_util::{pin_mut, StreamExt};
use streams_generator::stream_generator;

#[tokio::main]
async fn main() {
    let s = zero_to_three();
    pin_mut!(s);

    while let Some(value) = s.next().await {
        println!("got {}", value);
    }
}

#[stream_generator]
fn zero_to_three() -> impl Stream<Item = u32> {
    for i in 0..3 {
        tokio::time::sleep(Duration::from_secs(1)).await;
        yield i;
    }
}

// // The above function expands into the following:
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
