use crate::GeneratorImpl;
use std::{
    ops::{Generator, GeneratorState},
    pin::Pin,
};

impl<G> GeneratorImpl<G> {
    #[doc(hidden)]
    pub unsafe fn new_sync<Y, R>(generator: G) -> impl SyncGenerator<Y, R>
    where
        G: Generator<(), Yield = Y, Return = R>,
    {
        Self { generator }
    }
}

/// This trait is a combination of [`Iterator`], [`Finally`] and [`Generator`] all in one neat package.
pub trait SyncGenerator<Y, R>: Iterator<Item = Y> + Finally<Output = R> {
    /// Same as [`Generator::resume`] but with no argument, to match normal iterators
    fn resume(self: Pin<&mut Self>) -> GeneratorState<Y, R>;

    #[doc(hidden)]
    fn into_sync_generator(self) -> Self
    where
        Self: Sized,
    {
        self
    }
}

/// This allows synchronous generators a way to return a value
/// once the execution is complete.
pub trait Finally {
    /// Type to return
    type Output;
    /// Consume to get the output.
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
