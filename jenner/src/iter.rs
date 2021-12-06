use std::{ops::GeneratorState, pin::Pin};

use pin_project::pin_project;

use crate::{Finally, SyncGenerator};

#[doc(hidden)]
pub trait IntoSyncGenerator {
    type Yield;
    type Return;
    type SyncGenerator: SyncGenerator<Self::Yield, Self::Return>;
    fn into_sync_generator(self) -> Self::SyncGenerator;
}

impl<S> IntoSyncGenerator for S
where
    S: Iterator,
{
    type Yield = S::Item;
    type Return = ();
    type SyncGenerator = IterGenerator<Self>;

    fn into_sync_generator(self) -> Self::SyncGenerator {
        IterGenerator { iter: self }
    }
}

#[doc(hidden)]
#[pin_project]
pub struct IterGenerator<I> {
    iter: I,
}

impl<I> Iterator for IterGenerator<I>
where
    I: Iterator,
{
    type Item = I::Item;

    fn next(&mut self) -> Option<Self::Item> {
        self.iter.next()
    }
}

impl<I> Finally for IterGenerator<I>
where
    I: Iterator,
{
    type Output = ();
    fn finally(self) -> Self::Output {}
}

impl<S> SyncGenerator<S::Item, ()> for IterGenerator<S>
where
    S: Iterator,
{
    fn resume(self: Pin<&mut Self>) -> GeneratorState<S::Item, ()> {
        match self.project().iter.next() {
            Some(r) => GeneratorState::Yielded(r),
            None => GeneratorState::Complete(()),
        }
    }
}
