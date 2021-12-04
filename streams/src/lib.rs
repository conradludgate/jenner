#![feature(generator_trait)]

use futures_core::{Future, Stream};
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
pub unsafe fn new_stream_generator<Y, R, G>(generator: G) -> impl StreamGenerator<Y, R>
where
    G: Generator<UnsafeContextRef, Yield = Poll<Y>, Return = R>,
{
    AsyncStream { generator }
}

impl<Y, G> Stream for AsyncStream<G>
where
    G: Generator<UnsafeContextRef, Yield = Poll<Y>>,
{
    type Item = Y;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        match self.project().generator.resume(cx.into()) {
            GeneratorState::Yielded(p) => p.map(Some),
            GeneratorState::Complete(_) => Poll::Ready(None),
        }
    }
}

/// Future evaluates to the return value of the async stream
impl<R, G> Future for AsyncStream<G>
where
    G: Generator<UnsafeContextRef, Return = R>,
{
    type Output = R;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        match self.project().generator.resume(cx.into()) {
            GeneratorState::Yielded(_) => Poll::Pending,
            GeneratorState::Complete(r) => Poll::Ready(r),
        }
    }
}

impl<Y, R, G> StreamGenerator<Y, R> for AsyncStream<G>
where
    G: Generator<UnsafeContextRef, Yield = Poll<Y>, Return = R>,
{
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

pub trait StreamGenerator<Y, R>: Stream<Item = Y> + Future<Output = R> {
    fn poll_resume(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<GeneratorState<Y, R>>;
}
