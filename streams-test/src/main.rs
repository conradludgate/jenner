use std::{
    future::Future,
    ops::Range,
    pin::Pin,
    task::{Context, Poll},
    time::Duration,
};

use futures_core::Stream;
use streams_generator::{
    loops::{ForLoop, GenLoopState, StreamGeneratorLoop, StreamGeneratorLoopBody, GenLoopInnerState},
    stream_generator, GenState, StreamGenerator,
};
use futures_util::{pin_mut, StreamExt};

#[tokio::main]
async fn main() {
    let s = foo();
    pin_mut!(s);

    while let Some(value) = s.next().await {
        println!("got {}", value);
    }
}

fn foo() -> impl Stream<Item = i32> {
    // Ideally, the following stream_generator invocation would expand to the following FSM

    // stream_generator!{
    //     yield 10;

    //     for i in 0..10 {
    //         tokio::time::sleep(Duration::from_secs(1)).await;
    //         yield i
    //     }

    //     yield 0;
    // }

    use ::streams_generator::pin_project;

    #[pin_project(project = __StreamGenerator_FiniteStateMachine1_Proj)]
    enum __StreamGenerator_FiniteStateMachine1 {
        Yield1(),
        InnerStream1(#[pin] ForLoop<(), Range<i32>, __StreamGenerator_FiniteStateMachine2>),
        Yield2(),
        Return1(),
    }

    impl StreamGenerator for __StreamGenerator_FiniteStateMachine1 {
        type Yield = i32;
        type Return = ();

        fn poll_resume(
            mut self: Pin<&mut Self>,
            cx: &mut Context<'_>,
        ) -> Poll<GenState<Self::Yield, Self::Return>> {
            match self.as_mut().project() {
                __StreamGenerator_FiniteStateMachine1_Proj::Yield1() => {
                    let result = Poll::Ready(GenState::Yield(10));

                    self.set(__StreamGenerator_FiniteStateMachine1::InnerStream1(
                        ForLoop::init(((), 0..10)),
                    ));

                    result
                }
                __StreamGenerator_FiniteStateMachine1_Proj::InnerStream1(inner) => {
                    match inner.poll_loop(cx) {
                        Poll::Ready(GenLoopState::Yield(y)) => Poll::Ready(GenState::Yield(y)),
                        Poll::Ready(GenLoopState::Break(())) => {
                            self.set(__StreamGenerator_FiniteStateMachine1::Yield2());
                            self.poll_resume(cx)
                        }
                        Poll::Ready(GenLoopState::Return(())) => {
                            self.set(__StreamGenerator_FiniteStateMachine1::Return1());
                            self.poll_resume(cx)
                        }
                        Poll::Pending => Poll::Pending,
                    }
                }
                __StreamGenerator_FiniteStateMachine1_Proj::Yield2() => {
                    let result = Poll::Ready(GenState::Yield(0));

                    self.set(__StreamGenerator_FiniteStateMachine1::Return1());

                    result
                }
                __StreamGenerator_FiniteStateMachine1_Proj::Return1() => {
                    Poll::Ready(GenState::Return(()))
                }
            }
        }
    }

    #[pin_project(project = __StreamGenerator_FiniteStateMachine2_Proj)]
    enum __StreamGenerator_FiniteStateMachine2 {
        Await1(i32),
        AwaitFut1(i32, #[pin] tokio::time::Sleep),
        Yield1(i32),
        Continue(),
    }

    impl StreamGeneratorLoopBody for __StreamGenerator_FiniteStateMachine2 {
        type Ctx = ((), i32);
        fn init(ctx: Self::Ctx) -> Self {
            __StreamGenerator_FiniteStateMachine2::Await1(ctx.1)
        }

        type Yield = i32;
        type Return = ();
        type Break = ();

        fn poll_loop_body(
            mut self: Pin<&mut Self>,
            cx: &mut Context<'_>,
        ) -> Poll<GenLoopInnerState<Self::Yield, Self::Break, Self::Return>>
        {
            match self.as_mut().project() {
                __StreamGenerator_FiniteStateMachine2_Proj::Await1(i) => {
                    let fut = tokio::time::sleep(Duration::from_secs(1));

                    let inner = __StreamGenerator_FiniteStateMachine2::AwaitFut1(*i, fut);
                    self.set(inner);

                    self.poll_loop_body(cx)
                },
                __StreamGenerator_FiniteStateMachine2_Proj::AwaitFut1(i, fut) => {
                    match fut.poll(cx) {
                        Poll::Ready(()) => {
                            let inner = __StreamGenerator_FiniteStateMachine2::Yield1(*i);
                            self.set(inner);
                            self.poll_loop_body(cx)
                        },
                        Poll::Pending => Poll::Pending,
                    }
                },
                __StreamGenerator_FiniteStateMachine2_Proj::Yield1(i) => {
                    let result = Poll::Ready(GenLoopInnerState::Yield(*i));

                    let inner = __StreamGenerator_FiniteStateMachine2::Continue();
                    self.set(inner);

                    result
                },
                __StreamGenerator_FiniteStateMachine2_Proj::Continue() => {
                    Poll::Ready(GenLoopInnerState::Continue)
                },
            }
        }
    }

    streams_generator::Stream::new(__StreamGenerator_FiniteStateMachine1::Yield1())
}
