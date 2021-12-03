pub use pin_project::pin_project;
use std::{
    pin::Pin,
    task::{Context, Poll},
};
pub use streams_generator_macro::stream_generator;

mod for_loop;
pub use for_loop::*;

#[doc(hidden)]
pub trait StreamGeneratorLoop {
    type Ctx;
    fn init(ctx: Self::Ctx) -> Self;

    type Yield;
    type Break;
    type Return;
    fn poll_loop(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<GenLoopState<Self::Yield, Self::Break, Self::Return>>;
}

pub enum GenLoopState<Y, B, R> {
    Yield(Y),
    Return(R),
    Break(B),
}

#[doc(hidden)]
pub trait StreamGeneratorLoopBody {
    type Ctx;
    fn init(ctx: Self::Ctx) -> Self;

    type Yield;
    type Break;
    type Return;
    fn poll_loop_body(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<GenLoopInnerState<Self::Yield, Self::Break, Self::Return>>;
}

pub enum GenLoopInnerState<Y, B, R> {
    Yield(Y),
    Return(R),
    Break(B),
    Continue,
}
