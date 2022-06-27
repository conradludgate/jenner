use std::{
    async_iter::AsyncIterator,
    future::Future,
    ops::GeneratorState,
    pin::Pin,
    task::{Context, Poll},
};

use crate::AsyncGenerator;

#[doc(hidden)]
pub trait IntoAsyncGenerator {
    type Yield;
    type Return;
    type AsyncGenerator: AsyncGenerator<Self::Yield, Self::Return>;
    fn into_async_generator(self) -> Self::AsyncGenerator;
}

impl<S> IntoAsyncGenerator for S
where
    S: AsyncIterator,
{
    type Yield = S::Item;
    type Return = ();
    type AsyncGenerator = StreamGenerator<Self>;

    fn into_async_generator(self) -> Self::AsyncGenerator {
        StreamGenerator { stream: self }
    }
}

#[doc(hidden)]
pub struct StreamGenerator<S> {
    stream: S,
}
impl<S> Unpin for StreamGenerator<S> {}

impl<S> StreamGenerator<S> {
    fn project_stream(self: Pin<&mut Self>) -> Pin<&mut S> {
        let Self { stream } = self.get_mut();
        unsafe { Pin::new_unchecked(stream) }
    }
}

impl<S> AsyncIterator for StreamGenerator<S>
where
    S: AsyncIterator,
{
    type Item = S::Item;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        self.project_stream().poll_next(cx)
    }
}

impl<S> Future for StreamGenerator<S>
where
    S: AsyncIterator,
{
    type Output = ();

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        match self.project_stream().poll_next(cx) {
            Poll::Pending | Poll::Ready(Some(_)) => Poll::Pending,
            Poll::Ready(None) => Poll::Ready(()),
        }
    }
}

impl<S> AsyncGenerator<S::Item, ()> for StreamGenerator<S>
where
    S: AsyncIterator,
{
    fn poll_resume(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<GeneratorState<S::Item, ()>> {
        match self.project_stream().poll_next(cx) {
            Poll::Pending => Poll::Pending,
            Poll::Ready(Some(r)) => Poll::Ready(GeneratorState::Yielded(r)),
            Poll::Ready(None) => Poll::Ready(GeneratorState::Complete(())),
        }
    }
}
