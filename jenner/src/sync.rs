use effective::{Blocking, EffectResult, Effective, EffectiveResult, Failure, Multiple};

use std::{
    convert::Infallible,
    ops::{Generator, GeneratorState},
    pin::Pin,
};

pin_project_lite::pin_project!(
    #[doc(hidden)]
    pub struct SyncGeneratorImpl<G> {
        #[pin]
        generator: G,
    }
);

pin_project_lite::pin_project!(
    #[doc(hidden)]
    pub struct SyncFallibleGeneratorImpl<G> {
        #[pin]
        generator: G,
    }
);

impl<G> SyncGeneratorImpl<G> {
    #[doc(hidden)]
    pub fn create<Y>(
        generator: G,
    ) -> impl Effective<Item = Y, Produces = Multiple, Failure = Infallible, Async = Blocking>
    where
        G: Generator<(), Yield = Y, Return = ()>,
    {
        Self { generator }
    }
}

impl<G> SyncFallibleGeneratorImpl<G> {
    #[doc(hidden)]
    pub fn create<Y, E>(
        generator: G,
    ) -> impl Effective<Item = Y, Produces = Multiple, Failure = Failure<E>, Async = Blocking>
    where
        G: Generator<(), Yield = Y, Return = Result<(), E>>,
    {
        Self { generator }
    }
}

impl<G> Effective for SyncGeneratorImpl<G>
where
    G: Generator<(), Return = ()>,
{
    type Item = G::Yield;
    type Failure = Infallible;
    type Produces = Multiple;
    type Async = Blocking;

    fn poll_effect(
        self: Pin<&mut Self>,
        _cx: &mut std::task::Context<'_>,
    ) -> EffectiveResult<Self> {
        match self.project().generator.resume(()) {
            GeneratorState::Yielded(x) => EffectResult::Item(x),
            GeneratorState::Complete(()) => EffectResult::Done(Multiple),
        }
    }
}

impl<G, E> Effective for SyncFallibleGeneratorImpl<G>
where
    G: Generator<(), Return = Result<(), E>>,
{
    type Item = G::Yield;
    type Failure = Failure<E>;
    type Produces = Multiple;
    type Async = Blocking;

    fn poll_effect(
        self: Pin<&mut Self>,
        _cx: &mut std::task::Context<'_>,
    ) -> EffectiveResult<Self> {
        match self.project().generator.resume(()) {
            GeneratorState::Yielded(x) => EffectResult::Item(x),
            GeneratorState::Complete(Err(e)) => EffectResult::Failure(Failure(e)),
            GeneratorState::Complete(Ok(())) => EffectResult::Done(Multiple),
        }
    }
}
