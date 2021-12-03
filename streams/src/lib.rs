#![feature(generator_trait)]

use futures_core::Stream;
pub use pin_project::pin_project;
use std::{
    mem,
    ops::{Generator, GeneratorState},
    pin::Pin,
    ptr::NonNull,
    task::{Context, Poll},
};
pub use streams_generator_macro::stream;

#[doc(hidden)]
pub struct UnsafeContextRef(NonNull<Context<'static>>);

impl UnsafeContextRef {
    #[doc(hidden)]
    pub unsafe fn get_context(&mut self) -> &mut Context<'_> {
        mem::transmute(self.0)
    }
}

impl<'a> From<&mut Context<'a>> for UnsafeContextRef {
    fn from(cx: &mut Context<'a>) -> Self {
        Self(unsafe { mem::transmute(cx) })
    }
}

unsafe impl Send for UnsafeContextRef {}

#[pin_project]
struct AsyncStream<G> {
    #[pin]
    generator: G,
}

#[doc(hidden)]
pub unsafe fn new_stream<T, G>(generator: G) -> impl Stream<Item = T>
where
    G: Generator<UnsafeContextRef, Yield = Poll<T>, Return = ()>,
{
    AsyncStream { generator }
}

impl<T, G> Stream for AsyncStream<G>
where
    G: Generator<UnsafeContextRef, Yield = Poll<T>, Return = ()>,
{
    type Item = T;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        match self.project().generator.resume(cx.into()) {
            GeneratorState::Yielded(p) => p.map(Some),
            GeneratorState::Complete(()) => Poll::Ready(None),
        }
    }
}

impl<Y, G> StreamGenerator for AsyncStream<G>
where
    G: Generator<UnsafeContextRef, Yield = Poll<Y>>,
{
    type Yield = Y;
    type Return = G::Return;

    fn poll_resume(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<GeneratorState<Y, G::Return>> {
        match self.project().generator.resume(cx.into()) {
            GeneratorState::Yielded(p) => p.map(GeneratorState::Yielded),
            GeneratorState::Complete(r) => Poll::Ready(GeneratorState::Complete(r)),
        }
    }
}

pub trait StreamGenerator {
    type Yield;
    type Return;
    fn poll_resume(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<GeneratorState<Self::Yield, Self::Return>>;
}



// #[macro_export]
// macro_rules! for_await {
//     ($i:ident in $stream:expr => { $body:tt }) => {
//         let mut $stream = $stream;
//         loop {
//             let $stream = unsafe {
//                 ::std::pin::Pin::new_unchecked(&mut $stream)
//             };
//             let $i = match ::futures_core::StreamExt::next($stream).await {
//                 Some($i) => $i,
//                 None => break;
//             }
//             {
//                 $body
//             }
//         }
//     };
// }
