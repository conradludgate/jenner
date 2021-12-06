use std::{pin::Pin, task::{Context, Poll}, ops::GeneratorState};

use futures_core::{Stream, Future};
use pin_project::pin_project;

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
    S: Stream,
{
    type Yield = S::Item;
    type Return = ();
    type AsyncGenerator = StreamGenerator<Self>;

    fn into_async_generator(self) -> Self::AsyncGenerator {
        StreamGenerator { stream: self }
    }
}

#[doc(hidden)]
#[pin_project]
pub struct StreamGenerator<S> {
    #[pin]
    stream: S,
}

impl<S> Stream for StreamGenerator<S>
where
    S: Stream,
{
    type Item = S::Item;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        self.project().stream.poll_next(cx)
    }
}

impl<S> Future for StreamGenerator<S>
where
    S: Stream,
{
    type Output = ();

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        match self.project().stream.poll_next(cx) {
            Poll::Pending | Poll::Ready(Some(_)) => Poll::Pending,
            Poll::Ready(None) => Poll::Ready(()),
        }
    }
}

impl<S> AsyncGenerator<S::Item, ()> for StreamGenerator<S>
where
    S: Stream,
{
    fn poll_resume(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<GeneratorState<S::Item, ()>> {
        match self.project().stream.poll_next(cx) {
            Poll::Pending => Poll::Pending,
            Poll::Ready(Some(r)) => Poll::Ready(GeneratorState::Yielded(r)),
            Poll::Ready(None) => Poll::Ready(GeneratorState::Complete(())),
        }
    }
}
