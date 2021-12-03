pub use pin_project::pin_project;
use std::{
    pin::Pin,
    task::{Context, Poll},
};
pub use streams_generator_macro::stream_generator;

use super::{GenLoopInnerState, GenLoopState, StreamGeneratorLoop, StreamGeneratorLoopBody};

#[pin_project]
pub struct ForLoop<Ctx, I1: IntoIterator, L: StreamGeneratorLoopBody<Ctx = (Ctx, I1::Item), Break = ()>>
{
    ctx: Ctx,
    iterator: I1::IntoIter,
    #[pin]
    loop_state: ForLoopState<Ctx, I1::Item, L>,
}

#[pin_project(project = ForLoopStateProj)]
enum ForLoopState<Ctx, I, L: StreamGeneratorLoopBody<Ctx = (Ctx, I)>> {
    Next,
    Inside(#[pin] L),
}

impl<
        Ctx: Clone,
        I1: IntoIterator,
        L: StreamGeneratorLoopBody<Ctx = (Ctx, I1::Item), Break = ()>,
    > StreamGeneratorLoop for ForLoop<Ctx, I1, L>
{
    type Ctx = (Ctx, I1);

    fn init(ctx: Self::Ctx) -> Self {
        let (ctx, iterator) = ctx;
        Self {
            ctx,
            iterator: iterator.into_iter(),
            loop_state: ForLoopState::Next,
        }
    }

    type Yield = L::Yield;
    type Break = ();
    type Return = L::Return;

    fn poll_loop(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<GenLoopState<Self::Yield, Self::Break, Self::Return>> {
        let mut this = self.as_mut().project();
        match this.loop_state.as_mut().project() {
            ForLoopStateProj::Next => match this.iterator.next() {
                Some(i) => {
                    this.loop_state
                        .set(ForLoopState::Inside(L::init((this.ctx.clone(), i))));
                    self.poll_loop(cx)
                }
                None => Poll::Ready(GenLoopState::Break(())),
            },
            ForLoopStateProj::Inside(l) => match l.poll_loop_body(cx) {
                Poll::Ready(GenLoopInnerState::Break(())) => Poll::Ready(GenLoopState::Break(())),
                Poll::Ready(GenLoopInnerState::Return(r)) => Poll::Ready(GenLoopState::Return(r)),
                Poll::Ready(GenLoopInnerState::Yield(y)) => Poll::Ready(GenLoopState::Yield(y)),
                Poll::Ready(GenLoopInnerState::Continue) => {
                    this.loop_state.set(ForLoopState::Next);
                    self.poll_loop(cx)
                }
                Poll::Pending => Poll::Pending,
            },
        }
    }
}
