#![feature(generator_trait)]

use futures_core::Stream;
pub use pin_project::pin_project;
use std::{
    mem,
    ops::Generator,
    pin::Pin,
    ptr::NonNull,
    task::{Context, Poll},
};
pub use streams_generator_macro::stream;

/// `Send`-able wrapper around a `*mut Context`
///
/// This exists to allow the generator inside a `FutureImpl` to be `Send`,
/// provided there are no other `!Send` things in the body of the generator.
#[doc(hidden)]
pub struct UnsafeContextRef(NonNull<Context<'static>>);

impl UnsafeContextRef {
    /// Get a reference to the wrapped context
    ///
    /// # Safety
    ///
    /// This must only be called from the `await!` macro within the
    /// `make_future` function, which will in turn only be run when the
    /// `FutureImpl` has been observed to be in a `Pin`, guaranteeing that the
    /// outer `*const` remains valid.
    // https://github.com/rust-lang/rust-clippy/issues/2906
    #[doc(hidden)]
    pub unsafe fn get_context(&mut self) -> &mut Context<'_> {
        unsafe fn reattach_context_lifetimes<'a>(
            context: NonNull<Context<'static>>,
        ) -> &'a mut Context<'a> {
            mem::transmute(context)
        }

        reattach_context_lifetimes(self.0)
    }
}

impl From<&mut Context<'_>> for UnsafeContextRef {
    fn from(cx: &mut Context<'_>) -> Self {
        fn eliminate_context_lifetimes(context: &mut Context<'_>) -> NonNull<Context<'static>> {
            unsafe { mem::transmute(context) }
        }

        UnsafeContextRef(eliminate_context_lifetimes(cx))
    }
}

unsafe impl Send for UnsafeContextRef {}

#[derive(Debug)]
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

impl<G> Stream for AsyncStream<G>
where
    G: Generator<UnsafeContextRef, Return = ()>,
    G::Yield: IsPoll,
{
    type Item = <G::Yield as IsPoll>::Ready;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        match self.project().generator.resume(cx.into()) {
            std::ops::GeneratorState::Yielded(p) => p.into_poll().map(Some),
            std::ops::GeneratorState::Complete(()) => Poll::Ready(None),
        }
    }
}

trait IsPoll {
    type Ready;

    fn into_poll(self) -> Poll<Self::Ready>;
}

impl<T> IsPoll for Poll<T> {
    type Ready = T;

    fn into_poll(self) -> Poll<<Self as IsPoll>::Ready> {
        self
    }
}
