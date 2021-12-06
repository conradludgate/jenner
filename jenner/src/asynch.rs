use std::{task::{Context, Poll}, ptr::NonNull, ops::{Generator, GeneratorState}, pin::Pin};
use futures_core::{Stream, Future};

use crate::GeneratorImpl;

#[doc(hidden)]
pub struct UnsafeContextRef(NonNull<Context<'static>>);

impl UnsafeContextRef {
    #[doc(hidden)]
    pub unsafe fn get_context(&mut self) -> &mut Context<'_> {
        std::mem::transmute(self.0)
    }
}

impl<'a> From<&mut Context<'a>> for UnsafeContextRef {
    fn from(cx: &mut Context<'a>) -> Self {
        Self(unsafe { std::mem::transmute(cx) })
    }
}

unsafe impl Send for UnsafeContextRef {}

impl<G> GeneratorImpl<G> {
    #[doc(hidden)]
    pub unsafe fn new_async<Y, R>(generator: G) -> impl AsyncGenerator<Y, R>
    where
        G: Generator<UnsafeContextRef, Yield = Poll<Y>, Return = R>,
    {
        Self { generator }
    }
}

/// This trait is a combination of [`Stream`], [`Future`] and [`Generator`] all in one neat package.
pub trait AsyncGenerator<Y, R>: Stream<Item = Y> + Future<Output = R> {
    /// Poll the async generator, resuming it's execution until the next yield or await.
    ///
    /// Possible outcomes:
    ///     `future.poll()` is `Pending` => returns `Poll::Pending`,
    ///     `future.poll()` is `Ready(_)` => execution of generator continues until next yield point,
    ///     `yield item;` => returns `Poll::Ready(GeneratorState::Yielded(item))`,
    ///     `return item;` => returns `Poll::Ready(GeneratorState::Completed(item))`,
    fn poll_resume(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<GeneratorState<Y, R>>;

    #[doc(hidden)]
    fn into_async_generator(self) -> Self
    where
        Self: Sized,
    {
        self
    }
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
