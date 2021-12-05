#![feature(generator_trait)]

use futures_core::{Future, Stream};
pub use jenner_macro::{async_generator, generator};
use pin_project::pin_project;
use std::{
    mem,
    ops::{Generator, GeneratorState},
    pin::Pin,
    ptr::NonNull,
    task::{Context, Poll},
};

pub mod exports {
    pub use futures_core::{Future, Stream};
    pub use std::{pin, task};
}

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
struct GeneratorImpl<G> {
    #[pin]
    generator: G,
}

#[doc(hidden)]
pub unsafe fn new_async_generator<Y, R, G>(generator: G) -> impl AsyncGenerator<Y, R>
where
    G: Generator<UnsafeContextRef, Yield = Poll<Y>, Return = R>,
{
    GeneratorImpl { generator }
}

#[doc(hidden)]
pub unsafe fn new_sync_generator<Y, R, G>(generator: G) -> impl SyncGenerator<Y, R>
where
    G: Generator<(), Yield = Y, Return = R>,
{
    GeneratorImpl { generator }
}

impl<Y, G> Stream for GeneratorImpl<G>
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
impl<R, G> Future for GeneratorImpl<G>
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

impl<Y, R, G> AsyncGenerator<Y, R> for GeneratorImpl<G>
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

pub trait AsyncGenerator<Y, R>: Stream<Item = Y> + Future<Output = R> {
    fn poll_resume(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<GeneratorState<Y, R>>;
}

pub trait SyncGenerator<Y, R>: Iterator<Item = Y> + Finally<Output = R> {
    fn resume(self: Pin<&mut Self>) -> GeneratorState<Y, R>;
}

pub trait Finally {
    type Output;
    fn finally(self) -> Self::Output;
}

impl<R, G> Finally for GeneratorImpl<G>
where
    G: Generator<(), Return = R>,
{
    type Output = R;

    fn finally(self) -> Self::Output {
        let mut gen = self.generator;
        loop {
            // SAFETY: since gen never moves during the lifetime of this loop
            // the pin assumptions are never violated during the usage of the generator
            let gen = unsafe { Pin::new_unchecked(&mut gen) };
            match gen.resume(()) {
                GeneratorState::Yielded(_) => (),
                GeneratorState::Complete(r) => break r,
            }
        }
    }
}

impl<Y, G> Iterator for GeneratorImpl<G>
where
    G: Generator<(), Yield = Y>,
{
    type Item = Y;

    fn next(&mut self) -> Option<Self::Item> {
        // TODO: validate the safety of this...
        let gen = unsafe { Pin::new_unchecked(&mut self.generator) };
        match gen.resume(()) {
            GeneratorState::Yielded(y) => Some(y),
            GeneratorState::Complete(_) => None,
        }
    }
}

impl<Y, R, G> SyncGenerator<Y, R> for GeneratorImpl<G>
where
    G: Generator<(), Yield = Y, Return = R>,
{
    fn resume(self: Pin<&mut Self>) -> GeneratorState<Y, R> {
        self.project().generator.resume(())
    }
}
