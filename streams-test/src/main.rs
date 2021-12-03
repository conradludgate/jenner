use std::{ops::Range, pin::Pin, task::{Context, Poll}, time::Duration, future::Future};

use streams_generator::{stream_generator, GenState, StreamGenerator, StreamGeneratorLoop};

fn main() {
    println!("Hello, world!");
}

fn foo() {
    // for loop is just another internal stream?

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
    enum __StreamGenerator_FiniteStateMachine1<SG1: StreamGeneratorLoop<(), Return = (), Yield = i32>> {
        Yield1(),
        InnerStream1(#[pin] SG1),
        Yield2(),
        Return1(),
    }

    struct __StreamGenerator_ForLoop1<I1: Iterator> {
        iterator: I1,
        fsm: __StreamGenerator_FiniteStateMachine2<I1::Item>,
    }

    #[pin_project(project = __StreamGenerator_FiniteStateMachine2_Proj)]
    enum __StreamGenerator_FiniteStateMachine2<i1> {
        Await1(i1),
        AwaitFut1(i1, #[pin] tokio::time::Sleep),
        Yield1(i1),
        Continue(),
    }

    impl<SG1: StreamGeneratorLoop<(), Return = (), Yield = i32>> StreamGenerator for __StreamGenerator_FiniteStateMachine1<SG1>
    {
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
                        SG1::init(()),
                    ));

                    result
                }
                __StreamGenerator_FiniteStateMachine1_Proj::InnerStream1(inner) => {
                    match inner.poll_loop(cx) {
                        Poll::Ready(GenState::Yield(y)) => Poll::Ready(GenState::Yield(y)),
                        Poll::Ready(GenState::Break(())) => {
                            self.set(__StreamGenerator_FiniteStateMachine1::Yield2());
                            self.poll_resume(cx)
                        }
                        Poll::Ready(GenState::Return(())) => {
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


    impl StreamGenerator for __StreamGenerator_FiniteStateMachine2<i32> {
        type Yield = i32;
        type Break = ();
        type Return = ();

        fn poll_resume(
            mut self: Pin<&mut Self>,
            cx: &mut Context<'_>,
        ) -> Poll<GenState<Self::Yield, Self::Break, Self::Return>> {
            match self.as_mut().project() {
                __StreamGenerator_FiniteStateMachine2_Proj::Await1(i) => {
                    let fut = tokio::time::sleep(Duration::from_secs(1));

                    self.set(__StreamGenerator_FiniteStateMachine2::AwaitFut1(*i, fut));
                    self.poll_resume(cx)
                },
                __StreamGenerator_FiniteStateMachine2_Proj::AwaitFut1(i, fut) => {
                    match fut.poll(cx) {
                        Poll::Ready(_) => todo!(),
                        Poll::Pending => todo!(),
                    }
                },
                __StreamGenerator_FiniteStateMachine2_Proj::Yield1(_) => todo!(),
                __StreamGenerator_FiniteStateMachine2_Proj::Continue() => todo!(),
            }
        }
    }
}

// for i in x {
//     ...
// }

// ->

// let mut x = x.into_iter();
// loop {
//     let i = match x.next() {
//         Some(i) => i,
//         None => break,
//     };
//     ...
// }
