use std::{
    convert::Infallible,
    future::Future,
    ops::{Coroutine, CoroutineState, DerefMut},
    pin::Pin,
    task::{Context, Poll},
};

use crate::{asynch::UnsafeContextRef, Stream};

/// A pinned asynchronous generator with distinct yielded and returned values.
///
/// `Pending` means that no value is ready; the generator must arrange to wake
/// the task when progress is possible. `Yielded` produces an item, and `Complete`
/// returns the final value. After completion or a panic, polling again may panic.
/// Implementations must remain memory-safe regardless of the polling order.
pub trait AsyncGenerator {
    /// Values produced before completion.
    type Yield;
    /// The final value, produced once on completion.
    type Return;

    /// Resume until an item, a final value, or a pending operation is reached.
    fn poll_resume(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<CoroutineState<Self::Yield, Self::Return>>;
}

/// An asynchronous generator that yields items and completes with `()`.
pub trait AsyncStream<T> = AsyncGenerator<Yield = T, Return = ()> + Stream<Item = T>;
/// An asynchronous generator that returns a value without yielding items.
pub trait AsyncValue<T> = AsyncGenerator<Yield = Infallible, Return = T> + Future<Output = T>;
/// A generator whose terminal error is exposed once as a stream item.
pub trait TryAsyncStream<T, E> =
    AsyncGenerator<Yield = T, Return = Result<(), E>> + Stream<Item = Result<T, E>>;
/// A synchronous generator that yields items and completes with `()`.
pub trait SyncStream<T> = Coroutine<(), Yield = T, Return = ()>;
/// A synchronous generator that returns a value without yielding items.
pub trait SyncValue<T> = Coroutine<(), Yield = Infallible, Return = T>;
/// A synchronous generator that yields items and may finish with an error.
pub trait TrySyncStream<T, E> = Coroutine<(), Yield = T, Return = Result<(), E>>;

impl<P> AsyncGenerator for Pin<P>
where
    P: DerefMut + Unpin,
    P::Target: AsyncGenerator,
{
    type Yield = <P::Target as AsyncGenerator>::Yield;
    type Return = <P::Target as AsyncGenerator>::Return;

    fn poll_resume(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<CoroutineState<Self::Yield, Self::Return>> {
        self.get_mut().as_mut().poll_resume(cx)
    }
}

pin_project_lite::pin_project! {
    #[doc(hidden)]
    pub struct Generator<G> {
        #[pin]
        coroutine: G,
    }
}

impl<G> Generator<G> {
    pub fn new(coroutine: G) -> Self {
        Self { coroutine }
    }
}

impl<G: Coroutine<()>> Coroutine<()> for Generator<G> {
    type Yield = G::Yield;
    type Return = G::Return;

    fn resume(self: Pin<&mut Self>, (): ()) -> CoroutineState<Self::Yield, Self::Return> {
        self.project().coroutine.resume(())
    }
}

impl<G, Y> AsyncGenerator for Generator<G>
where
    G: Coroutine<UnsafeContextRef, Yield = Poll<Y>>,
{
    type Yield = Y;
    type Return = G::Return;

    fn poll_resume(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<CoroutineState<Self::Yield, Self::Return>> {
        match self.project().coroutine.resume(cx.into()) {
            CoroutineState::Yielded(Poll::Ready(item)) => {
                Poll::Ready(CoroutineState::Yielded(item))
            }
            CoroutineState::Yielded(Poll::Pending) => Poll::Pending,
            CoroutineState::Complete(result) => Poll::Ready(CoroutineState::Complete(result)),
        }
    }
}

impl<G> Future for Generator<G>
where
    G: Coroutine<UnsafeContextRef, Yield = Poll<Infallible>>,
{
    type Output = G::Return;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        match self.poll_resume(cx) {
            Poll::Pending => Poll::Pending,
            Poll::Ready(CoroutineState::Yielded(never)) => match never {},
            Poll::Ready(CoroutineState::Complete(output)) => Poll::Ready(output),
        }
    }
}

impl<G, Y> Stream for Generator<G>
where
    G: Coroutine<UnsafeContextRef, Yield = Poll<Y>, Return = ()>,
{
    type Item = Y;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Y>> {
        self.poll_resume(cx).map(|state| match state {
            CoroutineState::Yielded(item) => Some(item),
            CoroutineState::Complete(()) => None,
        })
    }
}

pin_project_lite::pin_project! {
    #[doc(hidden)]
    pub struct TryAsyncStreamImpl<G> {
        #[pin]
        generator: G,
        done: bool,
    }
}

impl<G> TryAsyncStreamImpl<G> {
    pub fn new(generator: G) -> Self {
        Self {
            generator,
            done: false,
        }
    }
}

impl<G, E> AsyncGenerator for TryAsyncStreamImpl<G>
where
    G: AsyncGenerator<Return = Result<(), E>>,
{
    type Yield = G::Yield;
    type Return = Result<(), E>;

    fn poll_resume(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<CoroutineState<Self::Yield, Self::Return>> {
        let this = self.project();
        assert!(!*this.done, "generator resumed after completion");
        let state = this.generator.poll_resume(cx);
        if matches!(state, Poll::Ready(CoroutineState::Complete(_))) {
            *this.done = true;
        }
        state
    }
}

impl<G, E> Stream for TryAsyncStreamImpl<G>
where
    G: AsyncGenerator<Return = Result<(), E>>,
{
    type Item = Result<G::Yield, E>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        if self.done {
            return Poll::Ready(None);
        }
        self.poll_resume(cx).map(|state| match state {
            CoroutineState::Yielded(item) => Some(Ok(item)),
            CoroutineState::Complete(Ok(())) => None,
            CoroutineState::Complete(Err(error)) => Some(Err(error)),
        })
    }
}
