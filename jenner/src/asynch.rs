use std::{
    convert::Infallible,
    ops::{Generator, GeneratorState},
    pin::Pin,
    ptr::NonNull,
    task::{Context, Poll},
};

use effective::{Async, EffectResult, Effective, EffectiveResult, Failure, Multiple, Single};

pin_project_lite::pin_project!(
    #[doc(hidden)]
    pub struct AsyncGeneratorImpl<G> {
        #[pin]
        generator: G,
    }
);

pin_project_lite::pin_project!(
    #[doc(hidden)]
    pub struct AsyncFallibleGeneratorImpl<G> {
        #[pin]
        generator: G,
    }
);

pin_project_lite::pin_project!(
    #[doc(hidden)]
    pub struct AsyncImpl<G> {
        #[pin]
        generator: G,
    }
);

pin_project_lite::pin_project!(
    #[doc(hidden)]
    pub struct AsyncFallibleImpl<G> {
        #[pin]
        generator: G,
    }
);

#[doc(hidden)]
pub struct UnsafeContextRef(NonNull<Context<'static>>);

impl UnsafeContextRef {
    #[doc(hidden)]
    pub unsafe fn get_context<'a, 'b>(&mut self) -> &'a mut Context<'b> {
        std::mem::transmute(self.0)
    }
}

impl<'a> From<&mut Context<'a>> for UnsafeContextRef {
    fn from(cx: &mut Context<'a>) -> Self {
        Self(unsafe { std::mem::transmute(cx) })
    }
}

unsafe impl Send for UnsafeContextRef {}

impl<G> AsyncGeneratorImpl<G> {
    #[doc(hidden)]
    pub fn create<Y>(
        generator: G,
    ) -> impl Effective<Item = Y, Produces = Multiple, Failure = Infallible, Async = Async>
    where
        G: Generator<UnsafeContextRef, Yield = Poll<Y>, Return = ()>,
    {
        Self { generator }
    }
}

impl<G> AsyncFallibleGeneratorImpl<G> {
    #[doc(hidden)]
    pub fn create<Y, E>(
        generator: G,
    ) -> impl Effective<Item = Y, Produces = Multiple, Failure = Failure<E>, Async = Async>
    where
        G: Generator<UnsafeContextRef, Yield = Poll<Y>, Return = Result<(), E>>,
    {
        Self { generator }
    }
}

impl<G> AsyncImpl<G> {
    #[doc(hidden)]
    pub fn create<Y>(
        generator: G,
    ) -> impl Effective<Item = Y, Produces = Single, Failure = Infallible, Async = Async>
    where
        G: Generator<UnsafeContextRef, Yield = Poll<Infallible>, Return = Y>,
    {
        Self { generator }
    }
}

impl<G> AsyncFallibleImpl<G> {
    #[doc(hidden)]
    pub fn create<Y, E>(
        generator: G,
    ) -> impl Effective<Item = Y, Produces = Single, Failure = Failure<E>, Async = Async>
    where
        G: Generator<UnsafeContextRef, Yield = Poll<Infallible>, Return = Result<Y, E>>,
    {
        Self { generator }
    }
}

impl<Y, G> Effective for AsyncGeneratorImpl<G>
where
    G: Generator<UnsafeContextRef, Yield = Poll<Y>, Return = ()>,
{
    type Item = Y;
    type Failure = Infallible;
    type Produces = Multiple;
    type Async = Async;

    fn poll_effect(self: Pin<&mut Self>, cx: &mut Context<'_>) -> EffectiveResult<Self> {
        match self.project().generator.resume(cx.into()) {
            GeneratorState::Yielded(Poll::Ready(x)) => EffectResult::Item(x),
            GeneratorState::Yielded(Poll::Pending) => EffectResult::Pending(Async),
            GeneratorState::Complete(()) => EffectResult::Done(Multiple),
        }
    }
}

impl<Y, E, G> Effective for AsyncFallibleGeneratorImpl<G>
where
    G: Generator<UnsafeContextRef, Yield = Poll<Y>, Return = Result<(), E>>,
{
    type Item = Y;
    type Failure = Failure<E>;
    type Produces = Multiple;
    type Async = Async;

    fn poll_effect(self: Pin<&mut Self>, cx: &mut Context<'_>) -> EffectiveResult<Self> {
        match self.project().generator.resume(cx.into()) {
            GeneratorState::Yielded(Poll::Ready(x)) => EffectResult::Item(x),
            GeneratorState::Yielded(Poll::Pending) => EffectResult::Pending(Async),
            GeneratorState::Complete(Err(e)) => EffectResult::Failure(Failure(e)),
            GeneratorState::Complete(Ok(())) => EffectResult::Done(Multiple),
        }
    }
}

impl<Y, G> Effective for AsyncImpl<G>
where
    G: Generator<UnsafeContextRef, Yield = Poll<Infallible>, Return = Y>,
{
    type Item = Y;
    type Failure = Infallible;
    type Produces = Single;
    type Async = Async;

    fn poll_effect(self: Pin<&mut Self>, cx: &mut Context<'_>) -> EffectiveResult<Self> {
        match self.project().generator.resume(cx.into()) {
            GeneratorState::Yielded(Poll::Ready(_)) => unreachable!(),
            GeneratorState::Yielded(Poll::Pending) => EffectResult::Pending(Async),
            GeneratorState::Complete(x) => EffectResult::Item(x),
        }
    }
}

impl<Y, E, G> Effective for AsyncFallibleImpl<G>
where
    G: Generator<UnsafeContextRef, Yield = Poll<Infallible>, Return = Result<Y, E>>,
{
    type Item = Y;
    type Failure = Failure<E>;
    type Produces = Single;
    type Async = Async;

    fn poll_effect(self: Pin<&mut Self>, cx: &mut Context<'_>) -> EffectiveResult<Self> {
        match self.project().generator.resume(cx.into()) {
            GeneratorState::Yielded(Poll::Ready(_)) => unreachable!(),
            GeneratorState::Yielded(Poll::Pending) => EffectResult::Pending(Async),
            GeneratorState::Complete(Ok(x)) => EffectResult::Item(x),
            GeneratorState::Complete(Err(x)) => EffectResult::Failure(Failure(x)),
        }
    }
}
