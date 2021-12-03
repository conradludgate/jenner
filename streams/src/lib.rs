pub use pin_project::pin_project;
use std::{
    pin::Pin,
    task::{Context, Poll},
};
pub use streams_generator_macro::stream_generator;

pub mod loops;

// a combination of `Stream` and `Generator` concepts.
// allows both yield and return types
pub trait StreamGenerator {
    type Yield;
    type Return;
    fn poll_resume(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<GenState<Self::Yield, Self::Return>>;
}

pub enum GenState<Y, R> {
    Yield(Y),
    Return(R),
}

#[pin_project]
pub struct Stream<SG> {
    #[pin]
    generator: SG,
}

impl<SG> Stream<SG> {
    pub fn new(generator: SG) -> Self {
        Stream{generator}
    }
}

impl<SG> futures_core::Stream for Stream<SG>
where
    SG: StreamGenerator<Return = ()>,
{
    type Item = SG::Yield;

    fn poll_next(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<Self::Item>> {
        self.project()
            .generator
            .poll_resume(cx)
            .map(|state| match state {
                GenState::Yield(y) => Some(y),
                GenState::Return(()) => None,
            })
    }
}
