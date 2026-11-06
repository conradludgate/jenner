use std::{
    ops::{Coroutine, CoroutineState},
    pin::Pin,
};

/// Iterator adapters for a pinned synchronous generator.
pub trait SyncGeneratorExt: Coroutine<()> {
    /// Borrow a generator as an iterator without moving or allocating it.
    fn iter(self: Pin<&mut Self>) -> Iter<'_, Self>
    where
        Self: Coroutine<(), Return = ()>,
    {
        Iter { generator: self }
    }

    /// Yield each item as `Ok`, followed by a terminal `Err` if it fails.
    fn try_iter<E>(self: Pin<&mut Self>) -> TryIter<'_, Self>
    where
        Self: Coroutine<(), Return = Result<(), E>>,
    {
        TryIter {
            generator: self,
            done: false,
        }
    }
}

impl<G: Coroutine<()> + ?Sized> SyncGeneratorExt for G {}

/// An iterator borrowing a pinned generator; created by [`SyncGeneratorExt::iter`].
pub struct Iter<'a, G: ?Sized> {
    generator: Pin<&'a mut G>,
}

impl<G: Coroutine<(), Return = ()> + ?Sized> Iterator for Iter<'_, G> {
    type Item = G::Yield;

    fn next(&mut self) -> Option<Self::Item> {
        match self.generator.as_mut().resume(()) {
            CoroutineState::Yielded(item) => Some(item),
            CoroutineState::Complete(()) => None,
        }
    }
}

/// A fallible iterator borrowing a pinned generator.
///
/// Created by [`SyncGeneratorExt::try_iter`]. Its terminal error is returned
/// once, followed by `None` on subsequent calls.
pub struct TryIter<'a, G: ?Sized> {
    generator: Pin<&'a mut G>,
    done: bool,
}

impl<G, E> Iterator for TryIter<'_, G>
where
    G: Coroutine<(), Return = Result<(), E>> + ?Sized,
{
    type Item = Result<G::Yield, E>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.done {
            return None;
        }
        match self.generator.as_mut().resume(()) {
            CoroutineState::Yielded(item) => Some(Ok(item)),
            CoroutineState::Complete(result) => {
                self.done = true;
                result.err().map(Err)
            }
        }
    }
}
