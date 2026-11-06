use std::{
    convert::Infallible,
    ops::{Coroutine, CoroutineState},
    pin::Pin,
    ptr::NonNull,
    task::{Context, Poll},
};

use effective::{Async, EffectResult, Effective, Failure, Multiple, Single};

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
// This token may outlive a poll's Context. It must be replaced with the new
// resume argument before the coroutine dereferences it on a subsequent poll.
pub struct UnsafeContextRef(
    // Safety invariant: preserves the pointer obtained from a Context borrow;
    // its lifetime is erased, so it is not necessarily still live.
    NonNull<Context<'static>>,
);

impl UnsafeContextRef {
    #[doc(hidden)]
    /// # Safety
    ///
    /// The caller must use the token only in the poll that supplied it, on that
    /// poll's thread. Its Context and waker must remain live, and access to the
    /// Context must be exclusive for the returned borrow. Neither that borrow
    /// nor references obtained through it may escape the poll or cross a yield.
    pub unsafe fn get_context<'a, 'b>(&mut self) -> &'a mut Context<'b> {
        // SAFETY: From preserves the original Context pointer's alignment and
        // provenance. The caller guarantees it is still live on this thread,
        // that its contents remain valid, and that this borrow is exclusive.
        // The cast erases only the Context lifetime; the caller also confines
        // all uses of the resulting reference to the original active poll.
        unsafe { &mut *self.0.as_ptr().cast::<Context<'b>>() }
    }
}

impl<'a> From<&mut Context<'a>> for UnsafeContextRef {
    /// # Safety-usable invariant
    ///
    /// The token points to the same Context with unchanged alignment and
    /// provenance. This does not extend the Context's or its waker's lifetime.
    fn from(cx: &mut Context<'a>) -> Self {
        // Erase the lifetime on a raw pointer without creating a longer-lived
        // reference. Dereferencing it is restricted by get_context's contract.
        Self(NonNull::from(cx).cast())
    }
}

// SAFETY: Moving the opaque pointer does not access the non-Send Context.
// Fields are private and its only dereference is unsafe get_context, whose
// caller must restrict access to the original poll and thread. The generated
// coroutine replaces this token on resumption before dereferencing it again.
unsafe impl Send for UnsafeContextRef {}

impl<G> AsyncGeneratorImpl<G> {
    #[doc(hidden)]
    pub fn create<Y>(
        generator: G,
    ) -> impl Effective<Item = Y, Produces = Multiple, Failure = Infallible, Async = Async>
    where
        G: Coroutine<UnsafeContextRef, Yield = Poll<Y>, Return = ()>,
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
        G: Coroutine<UnsafeContextRef, Yield = Poll<Y>, Return = Result<(), E>>,
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
        G: Coroutine<UnsafeContextRef, Yield = Poll<Infallible>, Return = Y>,
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
        G: Coroutine<UnsafeContextRef, Yield = Poll<Infallible>, Return = Result<Y, E>>,
    {
        Self { generator }
    }
}

impl<Y, G> Effective for AsyncGeneratorImpl<G>
where
    G: Coroutine<UnsafeContextRef, Yield = Poll<Y>, Return = ()>,
{
    type Item = Y;
    type Failure = Infallible;
    type Produces = Multiple;
    type Async = Async;

    fn poll_effect(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> EffectResult<Self::Item, Self::Failure, Self::Produces, Self::Async> {
        match self.project().generator.resume(cx.into()) {
            CoroutineState::Yielded(Poll::Ready(x)) => EffectResult::Item(x),
            CoroutineState::Yielded(Poll::Pending) => EffectResult::Pending(Async),
            CoroutineState::Complete(()) => EffectResult::Done(Multiple),
        }
    }
}

impl<Y, E, G> Effective for AsyncFallibleGeneratorImpl<G>
where
    G: Coroutine<UnsafeContextRef, Yield = Poll<Y>, Return = Result<(), E>>,
{
    type Item = Y;
    type Failure = Failure<E>;
    type Produces = Multiple;
    type Async = Async;

    fn poll_effect(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> EffectResult<Self::Item, Self::Failure, Self::Produces, Self::Async> {
        match self.project().generator.resume(cx.into()) {
            CoroutineState::Yielded(Poll::Ready(x)) => EffectResult::Item(x),
            CoroutineState::Yielded(Poll::Pending) => EffectResult::Pending(Async),
            CoroutineState::Complete(Err(e)) => EffectResult::Failure(Failure(e)),
            CoroutineState::Complete(Ok(())) => EffectResult::Done(Multiple),
        }
    }
}

impl<Y, G> Effective for AsyncImpl<G>
where
    G: Coroutine<UnsafeContextRef, Yield = Poll<Infallible>, Return = Y>,
{
    type Item = Y;
    type Failure = Infallible;
    type Produces = Single;
    type Async = Async;

    fn poll_effect(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> EffectResult<Self::Item, Self::Failure, Self::Produces, Self::Async> {
        match self.project().generator.resume(cx.into()) {
            CoroutineState::Yielded(Poll::Ready(_)) => unreachable!(),
            CoroutineState::Yielded(Poll::Pending) => EffectResult::Pending(Async),
            CoroutineState::Complete(x) => EffectResult::Item(x),
        }
    }
}

impl<Y, E, G> Effective for AsyncFallibleImpl<G>
where
    G: Coroutine<UnsafeContextRef, Yield = Poll<Infallible>, Return = Result<Y, E>>,
{
    type Item = Y;
    type Failure = Failure<E>;
    type Produces = Single;
    type Async = Async;

    fn poll_effect(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> EffectResult<Self::Item, Self::Failure, Self::Produces, Self::Async> {
        match self.project().generator.resume(cx.into()) {
            CoroutineState::Yielded(Poll::Ready(_)) => unreachable!(),
            CoroutineState::Yielded(Poll::Pending) => EffectResult::Pending(Async),
            CoroutineState::Complete(Ok(x)) => EffectResult::Item(x),
            CoroutineState::Complete(Err(x)) => EffectResult::Failure(Failure(x)),
        }
    }
}
